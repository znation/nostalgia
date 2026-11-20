use super::browse::BrowseList;
use super::composers::{Composer, group_by_composer};
use super::genres::{Genre, group_by_genre};
use super::loading::{RequestGeneration, fetch_into, play_into};
use super::subscriptions::*;
use super::views::{BROWSE_LIST_ID, BROWSE_PAGE_HEIGHT, BROWSE_SEARCH_ID, BROWSE_VIEWS};
use super::years::{Year, group_by_year};
use super::*;
use crate::audio::{PlaybackFormat, Spectrum};
use crate::equalizer::{BAND_COUNT, CustomPreset, GAIN_MAX_DB, GAIN_MIN_DB, PresetChoice};
use crate::library::{Album, Artist, Playlist, Song};
use crate::sample_library::sample_library;
use crate::settings::Settings;
use crate::settings::seam::SettingsWriter;
use crate::state::TransportMode;
use crate::test_support::{
    AudioCall, FailingAudio, FailingCommand, PREVIEW_URL, RecordingAudio, StubTransport,
    assert_ids, assert_preview_calls, numbered_songs, recording_spectrum, rock_preset,
    sample_album, sample_artist, sample_playlist, sample_song, second_album_songs, sign_in_session,
    stepping_songs,
};

use std::sync::atomic::AtomicU64;
use std::time::Duration;

mod support;
use support::*;

mod auto_preset_tests;
mod balance_detent_tests;
mod browse_tests;
mod composers_tests;
mod count_footer_tests;
mod ctrl_click_tests;
mod delete_preset_tests;
mod fade_tests;
mod file_info_tests;
mod flat_lists_tests;
mod jump_file_tests;
mod jump_tests;
mod jump_time_tests;
mod loading_tests;
mod mixer_backend_tests;
mod mixer_drag_readout_tests;
mod mixer_readout_input_tests;
mod mixer_tests;
mod move_row_tests;
mod move_selection_tests;
mod overlay_tests;
mod panels_tests;
mod position_poll_tests;
mod position_tick_tests;
mod randomize_tests;
mod readouts_tests;
mod recently_added_tests;
mod recently_played_tests;
mod removal_tests;
mod remove_all_tests;
mod reverse_tests;
mod save_list_tests;
mod save_preset_panel_tests;
mod save_preset_validation_tests;
mod screen_tests;
mod scroll_tests;
mod search_tests;
mod seek_drag_tests;
mod seek_preview_tests;
mod seek_wheel_tests;
mod settings_tests;
mod shortcuts_tests;
mod slider_wheel_routing_tests;
mod slider_wheel_tests;
mod sort_tests;
mod spectrum_tests;
mod stepping_tests;
mod subscriptions_tests;
mod title_drag_tests;
mod transport_tests;
mod volume_flash_tests;
mod volume_input_tests;
mod window_tests;
mod years_tests;
