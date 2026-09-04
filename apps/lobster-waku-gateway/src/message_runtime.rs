use super::*;

const MAX_MESSAGE_TEXT_CHARS: usize = 2_000;

impl GatewayRuntime {
    pub(crate) fn normalize_message_text(raw: String) -> Result<String, String> {
        let text = raw.trim().to_string();
        if text.is_empty() {
            return Err("message text required".into());
        }
        if text.chars().count() > MAX_MESSAGE_TEXT_CHARS {
            return Err(format!(
                "message text too long: max {MAX_MESSAGE_TEXT_CHARS} chars"
            ));
        }
        Ok(text)
    }

    pub(crate) fn edit_shell_message(
        &mut self,
        request: EditShellMessageRequest,
    ) -> Result<EditShellMessageResponse, String> {
        let conversation_id = ConversationId(request.room_id);
        let message_id = MessageId(request.message_id);
        let actor = IdentityId(request.actor);
        let text = Self::normalize_message_text(request.text)?;
        let lookup_limit = self.history_limit.max(64);
        let original_has_attachment = self
            .timeline_store
            .recent_messages(&conversation_id, lookup_limit)
            .into_iter()
            .any(|entry| {
                entry.envelope.message_id.0 == message_id.0
                    && entry
                        .envelope
                        .body
                        .plain_text
                        .starts_with(attachment_runtime::ATTACHMENT_URL_PREFIX)
            });
        if original_has_attachment {
            return Err("attachment messages cannot be edited; recall and resend instead".into());
        }
        self.validate_authenticated_sender(&actor)?;
        let edited_at_ms = Self::now_ms();
        let entry = self
            .timeline_store
            .edit_message(
                &conversation_id,
                &message_id,
                actor.clone(),
                text.clone(),
                edited_at_ms,
            )
            .map_err(|error| format!("edit message failed: {error}"))?
            .ok_or_else(|| format!("message {} not found", message_id.0))?;
        if entry.envelope.sender != actor {
            return Err("only the original sender can edit this message".into());
        }
        Ok(EditShellMessageResponse {
            ok: true,
            conversation_id: conversation_id.0,
            message_id: message_id.0,
            edit_status: "edited".into(),
            edited_at_ms,
            edited_by: actor.0,
            text,
        })
    }

    pub(crate) fn recall_shell_message(
        &mut self,
        request: RecallShellMessageRequest,
    ) -> Result<RecallShellMessageResponse, String> {
        let conversation_id = ConversationId(request.room_id);
        let message_id = MessageId(request.message_id);
        let actor = IdentityId(request.actor);
        self.validate_authenticated_sender(&actor)?;
        let recalled_at_ms = Self::now_ms();
        let entry = self
            .timeline_store
            .recall_message(&conversation_id, &message_id, actor.clone(), recalled_at_ms)
            .map_err(|error| format!("recall message failed: {error}"))?
            .ok_or_else(|| format!("message {} not found", message_id.0))?;
        if entry.envelope.sender != actor {
            return Err("only the original sender can recall this message".into());
        }
        Ok(RecallShellMessageResponse {
            ok: true,
            conversation_id: conversation_id.0,
            message_id: message_id.0,
            recall_status: "recalled".into(),
            recalled_at_ms,
            recalled_by: actor.0,
        })
    }

    pub(crate) fn validate_authenticated_sender(&self, sender: &IdentityId) -> Result<(), String> {
        let sender_id = sender.0.trim();
        if sender_id.is_empty() || sender_id == "访客" {
            return Err("login required before sending messages".into());
        }
        if self.resident_portability_revoked(sender) {
            return Err("resident is sanctioned and cannot send messages".into());
        }
        Ok(())
    }

    pub(crate) fn validate_public_room_post(
        &self,
        conversation_id: &ConversationId,
        sender: &IdentityId,
    ) -> Result<(), String> {
        if conversation_id.0 == "room:world:lobby" {
            return Ok(());
        }
        let Some(room) = self.public_room_by_conversation_id(conversation_id) else {
            return if conversation_id.0.starts_with("room:") {
                Err(format!("unknown public room: {}", conversation_id.0))
            } else {
                Ok(())
            };
        };

        let membership = self.active_membership(&room.city_id, sender);
        let can_moderate_frozen_room = membership.is_some_and(|membership| {
            membership.role.has(CityPermission::FreezeRoom)
                || membership.role.has(CityPermission::PublishAnnouncement)
        });
        if room.frozen && !can_moderate_frozen_room {
            return Err(format!("room {} is frozen", room.slug));
        }

        if membership.is_some() {
            return Ok(());
        }

        if self.public_room_accepts_non_member_posts(room) {
            return Ok(());
        }

        Err(format!(
            "resident {} is not active in city {}",
            sender.0, room.city_id.0
        ))
    }

    fn public_room_accepts_non_member_posts(&self, room: &PublicRoomRecord) -> bool {
        let Some(city) = self.cities.get(&room.city_id) else {
            return false;
        };
        if city.profile.approval_required || !city.profile.public_room_discovery_enabled {
            return false;
        }
        let trust_state = Self::trust_state_from_records(&self.city_trust, &room.city_id);
        Self::city_is_mirror_visible(&city.profile, trust_state)
    }

    pub(crate) fn validate_direct_message_post(
        &self,
        conversation_id: &ConversationId,
        sender: &IdentityId,
    ) -> Result<(), String> {
        if !conversation_id.0.starts_with("dm:") {
            return Ok(());
        }

        let Some(conversation) = self
            .timeline_store
            .active_conversations()
            .into_iter()
            .find(|item| item.conversation_id == *conversation_id)
        else {
            return Err(format!(
                "unknown direct conversation: {}",
                conversation_id.0
            ));
        };

        if conversation.participants.iter().any(|item| item == sender) {
            Ok(())
        } else {
            Err(format!(
                "resident {} is not a participant in {}",
                sender.0, conversation_id.0
            ))
        }
    }

    pub(crate) fn validate_reply_reference(
        &self,
        conversation_id: &ConversationId,
        reply_to_message_id: Option<&MessageId>,
    ) -> Result<(), String> {
        let Some(reply_to_message_id) = reply_to_message_id else {
            return Ok(());
        };
        let reply_exists_in_room = self
            .timeline_store
            .export_messages(conversation_id)
            .into_iter()
            .any(|entry| {
                entry.archived_at_ms.is_none() && entry.envelope.message_id == *reply_to_message_id
            });
        if reply_exists_in_room {
            Ok(())
        } else {
            Err(format!(
                "reply target {} not found in {}",
                reply_to_message_id.0, conversation_id.0
            ))
        }
    }

    pub(crate) fn append_shell_message(
        &mut self,
        request: ShellMessageRequest,
    ) -> Result<ShellMessageResponse, String> {
        let timestamp_ms = Self::now_ms();
        let conversation_id = ConversationId(request.room_id);
        let sender = IdentityId(request.sender);
        let sender_id = sender.0.clone();
        let reply_to_message_id = request
            .reply_to_message_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| MessageId(value.to_string()));
        let reply_to_message_id_response = reply_to_message_id
            .as_ref()
            .map(|message_id| message_id.0.clone());
        let attachment_id = request
            .attachment_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());
        let text = match &attachment_id {
            Some(_) => {
                // Image messages may carry an empty caption; still enforce length.
                let caption = request.text.trim().to_string();
                if caption.chars().count() > MAX_MESSAGE_TEXT_CHARS {
                    return Err(format!(
                        "message text too long: max {MAX_MESSAGE_TEXT_CHARS} chars"
                    ));
                }
                caption
            }
            None => Self::normalize_message_text(request.text)?,
        };
        let attachment = match &attachment_id {
            Some(id) => {
                let projection = self
                    .attachment_projection(id)
                    .ok_or_else(|| format!("unknown attachment: {id}"))?;
                Some(projection)
            }
            None => None,
        };
        let plain_text = match &attachment_id {
            Some(id) => attachment_runtime::attachment_reference(id, &text),
            None => text.clone(),
        };
        self.validate_authenticated_sender(&sender)?;
        self.validate_public_room_post(&conversation_id, &sender)?;
        self.validate_direct_message_post(&conversation_id, &sender)?;
        self.validate_reply_reference(&conversation_id, reply_to_message_id.as_ref())?;
        let message_id = self.next_message_id();
        let response_text = text.clone();
        let message = MessageEnvelope {
            message_id: MessageId(message_id.clone()),
            conversation_id: conversation_id.clone(),
            sender,
            reply_to_message_id,
            sender_device: DeviceId(request.device_id.unwrap_or_else(|| "mobile-web".into())),
            sender_profile: ClientProfile::mobile_web(),
            payload_type: if attachment_id.is_some() {
                PayloadType::AttachmentRef
            } else {
                PayloadType::Text
            },
            body: MessageBody {
                preview: if attachment_id.is_some() {
                    if response_text.is_empty() {
                        "[图片]".into()
                    } else {
                        format!("[图片] {response_text}")
                    }
                } else {
                    response_text.clone()
                },
                plain_text,
                language_tag: request.language_tag.unwrap_or_else(|| "zh-CN".into()),
            },
            ciphertext: vec![],
            timestamp_ms,
            ephemeral: false,
        };
        self.publish_message(message)?;

        Ok(ShellMessageResponse {
            ok: true,
            conversation_id: conversation_id.0,
            message_id,
            reply_to_message_id: reply_to_message_id_response,
            attachment,
            delivered_at_ms: timestamp_ms,
            delivery_status: "delivered".into(),
            sender: sender_id,
            text: response_text,
        })
    }

    pub(crate) fn send_cli_message(
        &mut self,
        request: CliSendRequest,
    ) -> Result<CliSendResponse, String> {
        let timestamp_ms = Self::now_ms();
        let text = Self::normalize_message_text(request.text)?;
        let sender = match parse_cli_address(&request.from)? {
            CliAddress::User(identity) | CliAddress::Agent(identity) => identity,
            CliAddress::Room(_) => {
                return Err("cli sender must be user:<id> or agent:<id>".into());
            }
        };
        self.validate_authenticated_sender(&sender)?;

        let conversation_id = match parse_cli_address(&request.to)? {
            CliAddress::Room(conversation_id) => {
                self.validate_public_room_post(&conversation_id, &sender)?;
                conversation_id
            }
            CliAddress::User(peer) | CliAddress::Agent(peer) => {
                if sender == peer {
                    return Err("direct message requires two distinct identities".into());
                }
                let conversation_id = self.resolve_cli_direct_conversation_id(&sender, &peer);
                self.ensure_direct_conversation(&conversation_id, &[sender.clone(), peer])?;
                conversation_id
            }
        };

        let message_id = self.next_message_id();
        let message = MessageEnvelope {
            message_id: MessageId(message_id.clone()),
            conversation_id: conversation_id.clone(),
            sender,
            reply_to_message_id: None,
            sender_device: DeviceId(request.client_tag.unwrap_or_else(|| "lobster-cli".into())),
            sender_profile: ClientProfile::desktop_terminal(),
            payload_type: PayloadType::Text,
            body: MessageBody {
                preview: text.clone(),
                plain_text: text,
                language_tag: "zh-CN".into(),
            },
            ciphertext: vec![],
            timestamp_ms,
            ephemeral: false,
        };
        self.publish_message(message)?;

        Ok(CliSendResponse {
            ok: true,
            conversation_id: conversation_id.0,
            message_id,
            delivered_at_ms: timestamp_ms,
        })
    }
}
