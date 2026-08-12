import {
  hasAnyShellPayload,
  hasGatewayShellStatePayload,
  normalizeShellMessages,
} from "./shell-payload.js";

export function normalizeShellStateForState(payload, fallbackState = {}) {
  if (!hasAnyShellPayload(payload) && !hasGatewayShellStatePayload(payload)) {
    return structuredClone(fallbackState);
  }
  const contracts = contractConversationMap(payload);
  const legacyRooms = new Map(
    (Array.isArray(payload.rooms) ? payload.rooms : []).map((room) => [room?.id, room]),
  );
  const normalizedRooms =
    contracts.size > 0
      ? Array.from(contracts.values()).map((contractRoom) =>
          mergeRoomWithContract(legacyRooms.get(contractRoom.id) || {}, contractRoom),
        )
      : Array.from(legacyRooms.values()).map((room) => mergeRoomWithContract(room, contracts.get(room.id)));
  return {
    ...payload,
    rooms: normalizedRooms,
  };
}

export function contractConversationMap(payload) {
  const scenes = new Map(
    (payload?.scene_render?.scenes || []).map((scene) => [scene.conversation_id, scene]),
  );
  return new Map(
    (payload?.conversation_shell?.conversations || []).map((conversation) => {
      const scene = scenes.get(conversation.conversation_id) || {};
      return [
        conversation.conversation_id,
        {
          id: conversation.conversation_id,
          title: conversation.title || conversation.conversation_id,
          subtitle: conversation.subtitle || "",
          meta: conversation.meta || "",
          kind_hint: conversation.kind_hint || null,
          participant_label: conversation.participant_label || null,
          route_label: conversation.route_label || null,
          list_summary: conversation.list_summary || null,
          status_line: conversation.status_line || null,
          thread_headline: conversation.thread_headline || null,
          chat_status_summary: conversation.chat_status_summary || null,
          queue_summary: conversation.queue_summary || null,
          preview_text: conversation.preview_text || null,
          last_activity_label: conversation.last_activity_label || null,
          activity_time_label: conversation.activity_time_label || null,
          overview_summary: conversation.overview_summary || null,
          context_summary: conversation.context_summary || null,
          member_count: conversation.member_count ?? null,
          caretaker: conversation.caretaker || null,
          detail_card: conversation.detail_card || null,
          workflow: conversation.workflow || null,
          inline_actions: Array.isArray(conversation.inline_actions) ? conversation.inline_actions : [],
          scene_banner: scene.scene_banner || null,
          scene_summary: scene.scene_summary || null,
          room_variant: scene.room_variant || null,
          room_motif: scene.room_motif || null,
          image_layer: scene.image_layer || null,
          hotspot_layer: scene.hotspot_layer || null,
          stage_projection: scene.stage || null,
          portrait_projection: scene.portrait || null,
          messages: normalizeShellMessages(conversation.messages),
        },
      ];
    }),
  );
}

export function mergeRoomWithContract(room, contract) {
  if (!contract) {
    return {
      ...room,
      messages: normalizeShellMessages(room.messages),
    };
  }
  const normalizedRoom = {
    ...room,
    messages: normalizeShellMessages(room.messages),
  };
  return {
    ...normalizedRoom,
    id: normalizedRoom.id || contract.id,
    title: contract.title || normalizedRoom.title || contract.id,
    subtitle: contract.subtitle || normalizedRoom.subtitle || "",
    meta: contract.meta || normalizedRoom.meta || "",
    kind_hint: contract.kind_hint || normalizedRoom.kind_hint || null,
    participant_label: contract.participant_label || normalizedRoom.participant_label || null,
    route_label: contract.route_label || normalizedRoom.route_label || null,
    list_summary: contract.list_summary || normalizedRoom.list_summary || null,
    status_line: contract.status_line || normalizedRoom.status_line || null,
    thread_headline: contract.thread_headline || normalizedRoom.thread_headline || null,
    chat_status_summary: contract.chat_status_summary || normalizedRoom.chat_status_summary || null,
    queue_summary: contract.queue_summary || normalizedRoom.queue_summary || null,
    preview_text: contract.preview_text || normalizedRoom.preview_text || null,
    last_activity_label: contract.last_activity_label || normalizedRoom.last_activity_label || null,
    activity_time_label: contract.activity_time_label || normalizedRoom.activity_time_label || null,
    overview_summary: contract.overview_summary || normalizedRoom.overview_summary || null,
    context_summary: contract.context_summary || normalizedRoom.context_summary || null,
    member_count: contract.member_count ?? normalizedRoom.member_count ?? null,
    caretaker: contract.caretaker || normalizedRoom.caretaker || null,
    detail_card: contract.detail_card || normalizedRoom.detail_card || null,
    workflow: contract.workflow || normalizedRoom.workflow || null,
    inline_actions:
      (Array.isArray(contract.inline_actions) && contract.inline_actions.length
        ? contract.inline_actions
        : normalizedRoom.inline_actions) || [],
    scene_banner: contract.scene_banner || normalizedRoom.scene_banner || null,
    scene_summary: contract.scene_summary || normalizedRoom.scene_summary || null,
    room_variant: contract.room_variant || normalizedRoom.room_variant || null,
    room_motif: contract.room_motif || normalizedRoom.room_motif || null,
    image_layer: contract.image_layer || normalizedRoom.image_layer || null,
    hotspot_layer: contract.hotspot_layer || normalizedRoom.hotspot_layer || null,
    stage_projection: contract.stage_projection || normalizedRoom.stage_projection || null,
    portrait_projection: contract.portrait_projection || normalizedRoom.portrait_projection || null,
    messages:
      normalizedRoom.messages?.length
        ? normalizedRoom.messages
        : contract.messages,
  };
}

export function synthesizeRoomsFromContracts(payload) {
  return Array.from(contractConversationMap(payload).values()).map((conversation) =>
    mergeRoomWithContract({}, conversation),
  );
}

export function governanceFromWorldSnapshotBundle(bundle) {
  const payload = bundle?.payload;
  if (!payload?.governance?.world) return null;
  return {
    world: payload.governance.world,
    portability: payload.governance.portability,
    cities: payload.governance.cities || [],
    memberships: payload.governance.memberships || [],
    public_rooms: payload.governance.public_rooms || [],
    residents: Array.isArray(payload.residents) ? payload.residents : [],
    world_directory: payload.directory || null,
    world_mirror_sources: Array.isArray(payload.mirror_sources) ? payload.mirror_sources : [],
    world_square: Array.isArray(payload.square) ? payload.square : [],
    world_safety: payload.safety || null,
  };
}

export function governanceWithResidentsPayload(governance, residentsPayload) {
  if (!governance) return null;
  return {
    ...governance,
    residents: Array.isArray(residentsPayload)
      ? residentsPayload
      : (governance.residents || []),
  };
}

export function governanceFromWorldApiPayload(payload, residentsPayload) {
  if (!payload?.world) return null;
  return {
    world: payload.world,
    portability: payload.portability,
    cities: payload.cities || [],
    memberships: payload.memberships || [],
    public_rooms: payload.public_rooms || [],
    residents: Array.isArray(residentsPayload) ? residentsPayload : [],
    world_directory: null,
    world_mirror_sources: [],
    world_square: [],
    world_safety: null,
  };
}
