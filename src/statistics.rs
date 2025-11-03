//! # Statistics Module - Typing Performance Data Collection and Analysis
//!
//! This module provides comprehensive data structures and algorithms for collecting,
//! processing, and analyzing typing performance statistics in real-time during
//! typing sessions.
//!
//! ## Architecture Overview
//!
//! The statistics system follows a multi-layered architecture:
//!
#![doc = simple_mermaid::mermaid!("../diagrams/statistics_architecture.mmd")]
//!
//! ## Key Components
//!
//! - **Input**: Individual keystroke events with timing and correctness
//! - **Measurement**: Point-in-time snapshots of all metrics
//! - **TempStatistics**: Accumulates data during active typing
//! - **Statistics**: Final session summary with complete analysis
//! - **CounterData**: Tracks various typing event counters
//!
//! ## Data Flow
//!
//! 1. **Event Collection**: Each keystroke generates an `Input` event
//! 2. **Real-time Processing**: `TempStatistics` updates counters and metrics
//! 3. **Periodic Sampling**: `Measurement` snapshots taken at intervals
//! 4. **Session Finalization**: Complete `Statistics` generated at end
//!
//! ## Performance Considerations
//!
//! - Measurements are taken at configurable intervals to balance accuracy vs. performance
//! - Consistency calculations use efficient Welford's algorithm for numerical stability
//! - Error tracking uses HashMap for efficient character-specific analysis

use std::collections::HashMap;

pub use web_time::{Duration, Instant};

use crate::{
    CharacterResult, State, Timestamp, Word,
    config::Configuration,
    math::{Accuracy, Consistency, Ipm, Wpm},
};

/// Individual keystroke event with timing and correctness information
///
/// Used to build the complete history of typing activity for analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Input {
    /// Timestamp in seconds from session start
    pub timestamp: Timestamp,
    /// Character that was typed
    pub char: char,
    /// Whether the keystroke was correct, wrong, corrected, or deleted
    pub result: CharacterResult,
}

/// Point-in-time snapshot of all typing performance metrics
///
/// Measurements are taken at regular intervals during typing to track
/// performance changes over time and calculate consistency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    /// When this measurement was taken (seconds from session start)
    pub timestamp: Timestamp,
    /// Words per minute at this point in time
    pub wpm: Wpm,
    /// Inputs per minute at this point in time
    pub ipm: Ipm,
    /// Typing accuracy at this point in time
    pub accuracy: Accuracy,
    /// Typing consistency up to this point in time
    pub consistency: Consistency,
}

impl Measurement {
    /// Create a new measurement snapshot from current session data
    ///
    /// Calculates all performance metrics based on the current state of the typing session.
    /// Consistency is calculated using all previous measurements plus the current one.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(m) where m = number of previous measurements
    /// - Space complexity: O(m) for temporary Vec of WPM values during consistency calculation
    /// - Dominated by Welford's algorithm for standard deviation calculation
    ///
    /// # Parameters
    ///
    /// * `timestamp` - Current time in seconds from session start
    /// * `input_len` - Current length of the typed input
    /// * `previous_measurements` - All measurements taken so far in this session
    /// * `input_history` - Complete history of keystrokes
    /// * `adds` - Total number of characters added (not including deletions)
    /// * `errors` - Total number of errors made
    /// * `corrections` - Total number of corrections made
    pub fn new(
        timestamp: Timestamp,
        input_len: usize,
        previous_measurements: &[Measurement],
        input_history: &[Input],
        adds: usize,
        errors: usize,
        corrections: usize,
    ) -> Self {
        let minutes = timestamp / 60.0;

        let wpm = Wpm::calculate(input_history.len(), errors, corrections, minutes);
        let ipm = Ipm::calculate(adds, input_history.len(), minutes);
        let accuracy = Accuracy::calculate(input_len, errors, corrections);

        // Calculate consistency - create a temporary Vec with all WPM measurements
        let all_wpm_measurements: Vec<Wpm> = previous_measurements
            .iter()
            .map(|m| m.wpm)
            .chain(std::iter::once(wpm))
            .collect();

        let consistency = Consistency::calculate(&all_wpm_measurements);

        Self {
            timestamp,
            wpm,
            ipm,
            accuracy,
            consistency,
        }
    }
}

/// Comprehensive counters for all typing events and errors
///
/// Tracks various statistics needed for performance analysis and detailed feedback.
/// Used internally by TempStatistics to accumulate data during typing sessions.
#[derive(Default, Debug, Clone)]
pub struct CounterData {
    /// Number of errors for each character (for targeted practice)
    pub char_errors: HashMap<char, usize>,
    /// Number of errors for each word (for word-level analysis)
    pub word_errors: HashMap<Word, usize>,
    /// Total characters added to the input (excluding deletions)
    pub adds: usize,
    /// Total delete operations performed
    pub deletes: usize,
    /// Total number of incorrect characters typed
    pub errors: usize,
    /// Total number of correct characters typed
    pub corrects: usize,
    /// Total number of corrections made (fixing previous errors)
    pub corrections: usize,
    /// Number of times correct characters were deleted (typing inefficiency)
    pub wrong_deletes: usize,
}

/// Complete statistical analysis of a finished typing session
///
/// Contains final performance metrics, historical data, and detailed counters.
/// Generated by finalizing a TempStatistics after the typing session ends.
#[derive(Debug, Clone)]
pub struct Statistics {
    /// Final words per minute calculations (raw, corrected, actual)
    pub wpm: Wpm,
    /// Final inputs per minute calculations (raw, actual)
    pub ipm: Ipm,
    /// Final accuracy percentages (raw, actual)
    pub accuracy: Accuracy,
    /// Final consistency percentages and standard deviations
    pub consistency: Consistency,
    /// Total duration of the typing session
    pub duration: Duration,

    /// All measurements taken during the session (for trend analysis)
    pub measurements: Vec<Measurement>,
    /// Complete keystroke history (for detailed analysis)
    pub input_history: Vec<Input>,
    /// Detailed counters for all typing events
    pub counters: CounterData,
    /// The length of the target text (buffer size)
    ///
    /// This represents the total number of characters in the text that needs to be typed,
    /// not how many the user has actually typed.
    pub input_length: usize,
    /// Number of characters the user hasn't typed yet
    ///
    /// Calculated as: `text_length - current_position`
    ///
    /// For example, if the target text is "hello" (5 chars) and the user has typed
    /// "hel" (3 chars), then `missing_characters = 2`.
    pub missing_characters: usize,
}

/// Real-time statistics accumulator for active typing sessions
///
/// Collects and processes typing events as they occur, taking periodic measurements
/// for consistency analysis. Designed for efficient real-time updates during typing.
#[derive(Default, Debug, Clone)]
pub struct TempStatistics {
    /// Measurements taken at regular intervals during the session
    pub measurements: Vec<Measurement>,
    /// Complete history of every keystroke in the session
    pub input_history: Vec<Input>,
    /// Running counters for all typing events and errors
    pub counters: CounterData,
    /// Timestamp of the last measurement (for interval tracking)
    last_measurement: Option<Timestamp>,
}

impl TempStatistics {
    /// Process a new keystroke event and update all statistics
    ///
    /// Updates counters, adds to input history, and takes a measurement
    /// if enough time has elapsed since the last one.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(1) typical case, O(m) when taking measurements
    ///   where m = number of previous measurements taken in this session
    /// - Space complexity: O(1) per call (grows input history by 1)
    /// - Measurements are taken at intervals (default: 1 second)
    /// - For a t-second session with i-second intervals: m ≈ t/i measurements
    ///
    /// # Parameters
    ///
    /// * `char` - The character that was typed
    /// * `result` - Whether it was correct, wrong, corrected, or deleted
    /// * `input_len` - Current length of the input text
    /// * `elapsed` - Time elapsed since session start
    /// * `config` - Configuration including measurement interval
    pub fn update(
        &mut self,
        char: char,
        result: CharacterResult,
        input_len: usize,
        elapsed: Duration,
        config: &Configuration,
    ) {
        let timestamp = elapsed.as_secs_f64();
        // Update input history and counters
        self.update_from_result(char, result, timestamp);

        // Take measurement if enough time has elapsed
        if self.should_take_measurement(timestamp, config.measurement_interval_seconds) {
            self.take_measurement(timestamp, input_len);
        }
    }

    /// Check if enough time has elapsed to take a new measurement
    fn should_take_measurement(&self, current_timestamp: Timestamp, interval_seconds: f64) -> bool {
        match self.last_measurement {
            Some(last_timestamp) => current_timestamp - last_timestamp >= interval_seconds,
            None => current_timestamp >= interval_seconds,
        }
    }

    /// Take a measurement and update the last measurement timestamp
    fn take_measurement(&mut self, timestamp: Timestamp, input_len: usize) {
        let measurement = Measurement::new(
            timestamp,
            input_len,
            &self.measurements,
            &self.input_history,
            self.counters.adds,
            self.counters.errors,
            self.counters.corrections,
        );
        self.measurements.push(measurement);
        self.last_measurement = Some(timestamp);
    }

    /// Update counters and input history
    fn update_from_result(&mut self, char: char, result: CharacterResult, timestamp: Timestamp) {
        match result {
            CharacterResult::Deleted(state) => {
                self.counters.deletes += 1;
                if matches!(state, State::Correct | State::Corrected) {
                    self.counters.wrong_deletes += 1
                }
            }
            CharacterResult::Wrong => {
                self.counters.errors += 1;
                self.counters.adds += 1;
                *self.counters.char_errors.entry(char).or_insert(0) += 1;
            }
            CharacterResult::Corrected => {
                self.counters.corrections += 1;
                self.counters.adds += 1;
            }
            CharacterResult::Correct => {
                self.counters.corrects += 1;
                self.counters.adds += 1;
            }
        }
        self.input_history.push(Input {
            timestamp,
            char,
            result,
        });
    }

    /// Convert temporary statistics into final session statistics
    ///
    /// Calculates final metrics based on the complete session data and returns
    /// a comprehensive Statistics struct suitable for analysis and storage.
    ///
    /// # Parameters
    ///
    /// * `duration` - Total duration of the typing session
    /// * `text_length` - Length of the target text (buffer size)
    /// * `current_position` - How many characters the user has currently typed
    pub fn finalize(
        mut self,
        duration: Duration,
        text_length: usize,
        current_position: usize,
    ) -> Statistics {
        let total_time = duration.as_secs_f64();
        self.take_measurement(total_time, current_position);

        let missing_characters = text_length.saturating_sub(current_position);

        let Self {
            measurements,
            input_history,
            counters,
            ..
        } = self;

        // Safety: We will always have at least one measurement
        let Measurement {
            wpm,
            ipm,
            accuracy,
            consistency,
            ..
        } = measurements.last().copied().unwrap();

        Statistics {
            wpm,
            ipm,
            accuracy,
            consistency,
            duration,
            measurements,
            input_history,
            counters,
            input_length: text_length,
            missing_characters,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_length_and_missing_characters() {
        let mut temp_stats = TempStatistics::default();
        let config = Configuration::default();

        // Simulate typing "hello" (5 characters) but only getting to position 3
        // Target text: "hello" (5 chars)
        // User types: "hel" (3 chars, with some errors along the way)

        // Type 'h' correctly (position=1)
        temp_stats.update(
            'h',
            CharacterResult::Correct,
            1,
            Duration::from_secs(0),
            &config,
        );

        // Type 'e' correctly (position=2)
        temp_stats.update(
            'e',
            CharacterResult::Correct,
            2,
            Duration::from_secs(0),
            &config,
        );

        // Type 'x' wrong (position=3, but wrong char)
        temp_stats.update(
            'x',
            CharacterResult::Wrong,
            3,
            Duration::from_secs(0),
            &config,
        );

        // Delete 'x' (back to position=2)
        temp_stats.update(
            'x',
            CharacterResult::Deleted(crate::State::Wrong),
            2,
            Duration::from_secs(0),
            &config,
        );

        // Type 'l' corrected (position=3)
        temp_stats.update(
            'l',
            CharacterResult::Corrected,
            3,
            Duration::from_secs(1),
            &config,
        );

        // Finalize: target is 5 chars ("hello"), but user only typed 3 ("hel")
        let target_text_length = 5;
        let current_position = 3;
        let stats =
            temp_stats.finalize(Duration::from_secs(1), target_text_length, current_position);

        // Verify input_length is the target text length
        assert_eq!(
            stats.input_length, 5,
            "input_length should be the target text length"
        );

        // Verify missing_characters = target_length - current_position = 5 - 3 = 2
        assert_eq!(
            stats.missing_characters, 2,
            "missing_characters should be 2 (still need to type 'lo')"
        );

        // Verify input_history contains all keystrokes (including the error and delete)
        assert_eq!(
            stats.input_history.len(),
            5,
            "input_history should contain all 5 keystrokes"
        );
    }

    #[test]
    fn test_missing_characters_with_no_errors() {
        let mut temp_stats = TempStatistics::default();
        let config = Configuration::default();

        // Simulate typing "hi" perfectly with no errors and completing it
        temp_stats.update(
            'h',
            CharacterResult::Correct,
            1,
            Duration::from_secs(0),
            &config,
        );
        temp_stats.update(
            'i',
            CharacterResult::Correct,
            2,
            Duration::from_secs(1),
            &config,
        );

        let target_text_length = 2;
        let current_position = 2; // Completed typing
        let stats =
            temp_stats.finalize(Duration::from_secs(1), target_text_length, current_position);

        // With perfect typing and completion
        assert_eq!(
            stats.input_length, 2,
            "input_length should be target text length"
        );
        assert_eq!(
            stats.input_history.len(),
            2,
            "input_history should contain 2 keystrokes"
        );
        assert_eq!(
            stats.missing_characters, 0,
            "missing_characters should be 0 when fully typed"
        );
    }

    #[test]
    fn test_missing_characters_partial_completion() {
        let mut temp_stats = TempStatistics::default();
        let config = Configuration::default();

        // Target text is "hello" (5 chars) but user only types "he" (2 chars)
        temp_stats.update(
            'h',
            CharacterResult::Correct,
            1,
            Duration::from_secs(0),
            &config,
        );
        temp_stats.update(
            'e',
            CharacterResult::Correct,
            2,
            Duration::from_secs(1),
            &config,
        );

        let target_text_length = 5; // "hello"
        let current_position = 2; // Only typed "he"
        let stats =
            temp_stats.finalize(Duration::from_secs(1), target_text_length, current_position);

        assert_eq!(
            stats.input_length, 5,
            "input_length should be target text length"
        );
        assert_eq!(
            stats.input_history.len(),
            2,
            "input_history should contain 2 keystrokes"
        );
        assert_eq!(
            stats.missing_characters, 3,
            "missing_characters should be 3 (still need 'llo')"
        );
    }

    #[test]
    fn test_missing_characters_no_typing() {
        // Edge case: User hasn't typed anything yet
        let temp_stats = TempStatistics::default();

        let target_text_length = 10;
        let current_position = 0; // Haven't typed anything
        let stats =
            temp_stats.finalize(Duration::from_secs(0), target_text_length, current_position);

        assert_eq!(
            stats.input_length, 10,
            "input_length should be target text length"
        );
        assert_eq!(
            stats.input_history.len(),
            0,
            "input_history should be empty"
        );
        assert_eq!(
            stats.missing_characters, 10,
            "missing_characters should equal target length when nothing typed"
        );
    }
}
