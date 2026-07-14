use iced::Task;


use super::super::message::Message;
use super::super::App;

impl App {
    pub(crate) fn update_equalizer(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetEqEnabled(on) => {
                let _ = firmium_backend::commands::equalizer::set_eq_enabled(&self.backend.audio_player, on);
                self.eq_state = Some(firmium_backend::commands::equalizer::get_eq_state());
                Task::none()
            }
            Message::SetEqProfile(name) => {
                let device = self
                    .eq_state
                    .as_ref()
                    .and_then(|e| e.default_device.clone())
                    .unwrap_or_default();
                let _ = firmium_backend::commands::equalizer::set_eq_active_profile(&self.backend.audio_player, device, name);
                self.eq_state = Some(firmium_backend::commands::equalizer::get_eq_state());
                Task::none()
            }
            Message::EqBandChanged(idx, gain) => {
                if let Some(eq) = &mut self.eq_state {
                    if let Some(active) = eq.active_profile.clone() {
                        if let Some(p) = eq.profiles.iter_mut().find(|p| p.name == active) {
                            if let Some(b) = p.bands.get_mut(idx) {
                                b.gain = gain;
                            }
                            let bands = p.bands.clone();
                            let _ = firmium_backend::commands::equalizer::set_eq_bands(&self.backend.audio_player, active, bands);
                        }
                    }
                }
                Task::none()
            }
            Message::EqNewProfileInput(s) => {
                self.eq_new_profile_name = s;
                Task::none()
            }
            Message::SaveEqProfile => {
                let name = self.eq_new_profile_name.trim().to_string();
                if !name.is_empty() {
                    // Save the active profile's current bands under the new name.
                    if let Some(eq) = &self.eq_state {
                        if let Some(p) = eq
                            .active_profile
                            .as_ref()
                            .and_then(|a| eq.profiles.iter().find(|p| &p.name == a))
                        {
                            let _ = firmium_backend::commands::equalizer::save_eq_profile(
                                &self.backend.audio_player,
                                name,
                                p.kind.clone(),
                                p.bands.clone(),
                            );
                        }
                    }
                    self.eq_new_profile_name.clear();
                    self.eq_state = Some(firmium_backend::commands::equalizer::get_eq_state());
                }
                Task::none()
            }
            Message::DeleteEqProfile(name) => {
                let _ = firmium_backend::commands::equalizer::delete_eq_profile(&self.backend.audio_player, name);
                self.eq_state = Some(firmium_backend::commands::equalizer::get_eq_state());
                Task::none()
            }

            // ── Mix ─────────────────────────────────────────────────────────────
            _ => unreachable!(),
        }
    }
}
