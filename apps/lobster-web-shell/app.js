import { computeComposerAvailability } from "./composer-state.js";
import {
  caretakerPanelModel,
  caretakerStatusItems,
} from "./shell-caretaker-panel.js";
import {
  composerStatusState,
  initShellComposer,
  seedComposerFromQuickAction,
  syncComposerDraft,
  focusComposerInput,
  autoSizeComposerInput,
  ensureComposerTip,
  renderComposerHero,
  updateComposerContext,
  updateComposerTip,
  ensureComposerKeyBindings,
  triggerComposerKeyboardSubmit,
  handleComposerInputKeydown,
  handleComposerFormPointerdown,
  renderComposerMeta,
  gatewayUnavailableForComposer,
} from "./shell-composer.js";
import { createComposerSymbolController } from "./shell-composer-symbols.js";
import { applyAvatarStyle } from "./shell-avatar.js";
import {
  createChatDetailCardMetaRow,
  createDetailRow,
  createDetailSection,
  createDomNodeFromSpec,
  createLine,
  createMetaChip,
  createOverviewMetric,
  createPill,
  createStageChip,
  setDatasetFlag,
  setInlineStyle,
} from "./shell-dom-helpers.js";
import {
  gatewayErrorMessage,
  localizedRuntimeError,
} from "./shell-errors.js";
import {
  downloadContent,
  exportFileExtension,
  exportMimeType,
} from "./shell-export-utils.js";
import {
  gatewayJsonHeaders,
  gatewayQueryParam,
  gatewayShellEventsUrl as buildGatewayShellEventsUrl,
  gatewayShellStateUrl as buildGatewayShellStateUrl,
  resolveGatewayUrlCandidate,
} from "./shell-gateway.js";
import { createGatewayRealtimeController } from "./shell-gateway-realtime.js";
import { createGatewayPollingController } from "./shell-gateway-polling.js";
import { createGatewaySyncController } from "./shell-gateway-sync.js";
import {
  bindShellForegroundLifecycle,
  runShellStartup,
} from "./shell-lifecycle.js";
import {
  governanceStatusClassState,
  governanceStatusText,
} from "./shell-governance-status.js";
import { createWorldSurfaceRenderers } from "./shell-world-surfaces.js";
import { createGovernanceCitySurfaceRenderer } from "./shell-governance-city-surfaces.js";
import { createResidentSurfaceRenderer } from "./shell-resident-surfaces.js";
import { createRoomListSurfaceRenderer } from "./shell-room-list-surfaces.js";
import { createRoomDigestSurfaceRenderer } from "./shell-room-digest-surfaces.js";
import { createThreadStatusSurfaceRenderer } from "./shell-thread-status-surfaces.js";
import {
  directSessionOpenRequestState,
  residentPrivateRoomAccessPromptModel,
} from "./shell-governance-render.js";
import { privateRoomLockedCardModel } from "./shell-private-room-locked-card.js";
import {
  displayCityDescription,
  displayCityTitle,
  translateFederationPolicy,
  translateMembershipState,
  translateProviderHealth,
  translateProviderMode,
  translateRoomKind,
  translateRoomKindForShellPage,
  translateSourceKind,
  translateTrustState,
} from "./shell-labels.js";
import {
  appliedPersonalRoomAccessPolicy,
  personalRoomAccessPolicyControlState,
  personalRoomAccessPolicySubmitRequestState,
} from "./shell-personal-room-policy.js";
import {
  createPendingMessageEchoStore,
  visiblePendingEchoesForRoomData,
} from "./shell-message-state.js";
import { createMessageSendController } from "./shell-message-send.js";
import {
  buildReplyPreview,
  escapeHtml,
  formatDateTime,
  isSystemSender,
  messageAvatarTone,
  messageOwnerActionSpecs,
  messageRoleLabel,
  timelineCommittedMessageRenderItems,
  timelineMessageFlowSpec,
  timelineMessageRowSpec,
  timelinePendingMessageRowSpec,
  timelineTypingIndicatorSpec,
  messageStableId,
  messageThreadKind,
  timelineMetaChips,
} from "./shell-message-render.js";
import {
  createMessageSearchController,
  mountMessageSearchChrome,
} from "./shell-message-search.js";
import { createMessageActionSheet } from "./shell-message-action-sheet.js";
import { compressImageFile } from "./shell-image-compress.js";
import {
  createAttachmentLightbox,
  wireAttachmentLightbox,
} from "./shell-attachment-lightbox.js";
import { initInstallHint } from "./shell-install-hint.js";
import { initPushClient } from "./shell-push-client.js";
import { installAttachmentErrorFallback } from "./shell-attachment-fallback.js";
import {
  messageBodyDomSpec,
  messageQuickActionChipSpec,
  messageQuickStateChipSpec,
} from "./shell-message-body.js";
import {
  quickActionContextCopy,
  quickActionDefaultSendLabel,
  quickActionDraftStatusCopy,
  quickActionFollowUpCopy,
  quickActionFollowUpLabel,
  quickActionIntensity,
  quickActionOverviewCtaLabel,
  quickActionOverviewSummary,
  quickActionStage,
  buildWorkflowProgressDomSpec,
  quickActionStateStages,
  quickActionSummary,
  quickActionTone,
  buildRoomQuickActionPillDomSpec,
  buildRoomInlineActionsModel,
  buildRoomInlineActionDomSpec,
  nextQuickActionState,
} from "./shell-quick-action-labels.js";
import {
  QUICK_ACTION_BLUEPRINTS,
  QUICK_ACTION_INLINE_FIELD_PRIORITY,
  QUICK_ACTION_INLINE_STATE_FIELD_PRIORITY,
  quickActionWorkflowTemplate as _quickActionWorkflowTemplate,
} from "./shell-quick-action-templates.js";
import {
  buildQuickActionInlinePreviewPanelRenderDomModel,
  quickActionPreviewClickableDomSpec,
  quickActionPreviewKeyActivates,
  buildQuickActionInlinePreviewPanelModel,
  buildQuickActionPreviewSummaryLineDomSpec,
  buildQuickActionPreviewCardModel,
  buildQuickActionPreviewCardRenderDomSpec,
  buildRoomQuickPreviewPillDomSpec,
  buildQuickActionPreviewModel,
  normalizeQuickActionFieldLabel,
  parseStructuredQuickActionMessage,
  quickActionSnapshotFromHistory,
  quickActionSnapshotHistoryFromRecord,
  quickActionInlinePreviewFields,
  quickActionPreviewDefaultFieldView,
  quickActionPreviewFieldViewLabel,
  quickActionPreviewHistoryDescription,
  quickActionPreviewHistoryLabel,
  quickActionPreviewHistorySummary,
  quickActionPreviewPrimaryField,
  quickActionPreviewPrimaryFieldText,
  quickActionPreviewRoundLabel,
  resolveQuickActionPreviewView,
  quickActionPreviewSelectedFieldView,
  quickActionPreviewSelectedSnapshotIndex,
  quickActionPreviewSelectedState,
  quickActionWorkflowStructured,
} from "./shell-quick-action-preview.js";
import {
  chatRuntimeDetailModelForState,
  composerContextItemsForState,
  composerHeroModelForState,
  composerMetaBaseStatus,
  composerMetaQuickHint,
  composerPlaceholderForState as resolveComposerPlaceholderForState,
  conversationOverviewBaseStatusPills,
  conversationOverviewCaretakerStatusPillModel,
  conversationOverviewContextModel,
  conversationOverviewDraftPill,
  conversationOverviewHeaderModel,
  conversationOverviewRuntimeStatusPills,
  roomLastActivity as _roomLastActivity,
  roomPreview as _roomPreview,
  threadStatusRailModelForState,
  userConversationStatusPills,
} from "./shell-room-render.js";
import {
  roomStagePortraitChipsForState,
  roomStagePortraitSummaryForState,
  roomStagePortraitTitleForState,
  roomStageSummaryForState,
  userRoomProjectionForState,
} from "./shell-room-stage.js";
import {
  quickActionAdvanceLabel,
  quickActionContract,
  quickActionContractStateTemplate,
  quickActionStructuredDraft,
  quickActionTemplate,
  quickActionWorkflowTemplate,
  resetRoomQuickActions,
  roomQuickAction,
  setRoomQuickAction,
  setStateGetter,
} from "./shell-quick-actions.js";
import { createQuickActionReaders } from "./shell-quick-action-reader.js";
import {
  hasAnyShellPayload,
  hasConversationShellPayload,
  hasGatewayShellStatePayload,
  humanMembership,
  joinOrFallback,
  normalizeShellMessages,
} from "./shell-payload.js";
import {
  governanceFromWorldApiPayload,
  governanceFromWorldSnapshotBundle,
  governanceWithResidentsPayload,
  normalizeShellStateForState,
} from "./shell-state-normalize.js";
import {
  loadChatFocusPreference as loadChatFocusPreferenceFromState,
  persistChatFocusPreference as persistChatFocusPreferenceInState,
  resolveWorkspace as resolveWorkspaceFromState,
  defaultChatPaneForViewport as defaultChatPaneForViewportFromState,
  resolveChatPaneMode as resolveChatPaneModeFromState,
  loadRoomReadMarkers as loadRoomReadMarkersFromState,
  persistRoomReadMarkersToStorage,
  loadRoomDrafts as loadRoomDraftsFromState,
  draftForRoom as draftForRoomFromState,
  roomHasDraft as roomHasDraftFromState,
  updateRoomDraft as updateRoomDraftInState,
  loadRoomQuickStates as loadRoomQuickStatesFromState,
  setRoomQuickState as setRoomQuickStateInState,
  loadRoomQuickSnapshots as loadRoomQuickSnapshotsFromState,
  setRoomQuickSnapshot as setRoomQuickSnapshotInState,
} from "./shell-state.js";
import { createChatFocusController } from "./shell-chat-focus.js";
import {
  localPreviewMessagesForEmptyRoom as buildLocalPreviewMessagesForEmptyRoom,
  shouldRenderTimelineSkeletonRows as shouldRenderTimelineSkeletonRowsForContext,
  timelineNoRoomEmptyStateSpec,
} from "./shell-timeline-empty-state.js";
import {
  initSceneRuntime,
} from "./shell-scene-runtime.js";
import {
  imageLayerUrlForState,
  presetImageLayerUrlForState,
  timeAdjustedRuntimeSceneUrlForState,
} from "./shell-scene-image-layer.js";
import { createAuthController } from "./shell-auth.js";
import {
  caretakerNotificationCount,
  caretakerPendingCount,
  caretakerProfile,
  caretakerStatusLine,
  detailCardProfile,
  inlineActionProfile,
  portraitProjection,
  stageProjection,
  workflowProfile,
} from "./shell-room-profiles.js";
import { userDetailCardProjectionForState } from "./shell-user-detail-card.js";
import { conversationCalloutModelForState } from "./shell-conversation-callout.js";
import {
  createConversationCalloutParagraphNode as _createConversationCalloutParagraphNode,
  renderConversationCalloutContent as _renderConversationCalloutContent,
} from "./shell-conversation-callout-render.js";
import {
  chatPriorityBadgeDefaultText,
  modeBannerText,
  chatQuickLinksTargets,
  ensureModeBannerDom,
  ensureConversationCalloutDom,
} from "./shell-chrome-text.js";
import {
  createRoomStageSideElement as _createRoomStageSideElement,
  createRoomStageCanvasChrome as _createRoomStageCanvasChrome,
  createChatDetailPanelChrome as _createChatDetailPanelChrome,
  renderRoomStagePortraitChrome as _renderRoomStagePortraitChrome,
} from "./shell-scene-chrome.js";
import {
  createCaretakerPanelTitleNode as _createCaretakerPanelTitleNode,
  createCaretakerPanelHeaderNode as _createCaretakerPanelHeaderNode,
  createCaretakerPanelSummaryNode as _createCaretakerPanelSummaryNode,
  createCaretakerMessageNode as _createCaretakerMessageNode,
  createCaretakerMessagesNode as _createCaretakerMessagesNode,
  createCaretakerRulesNode as _createCaretakerRulesNode,
  renderCaretakerPanelBody as _renderCaretakerPanelBody,
} from "./shell-caretaker-dom.js";
import {
  gatewayMessagePayloadForState,
  editMessagePayloadForState,
  recallMessagePayloadForState,
} from "./shell-message-action-payload.js";
import {
  applyLocalTimeOfDayState,
  availableWorkspacesForShellMode,
  currentShellPage,
  defaultIdentityForShellMode,
  defaultWorkspaceForShellMode,
  initThemeToggle,
  normalizeProviderConnectionState,
  providerIndicatesGatewayOffline,
  resolveShellMode,
  safeLocalStorageGet,
  safeLocalStorageSet,
  scopedShellStorageKey,
  setNodeText,
  shellModeConfig,
  translateDeliveryMode,
  translateProviderConnectionState,
  translateWorkspace,
  translateShellMode,
} from "./shell-shared.js";
import {
  shellModeViewState as _shellModeViewState,
  applyShellModeBodyDataset as _applyShellModeBodyDataset,
  updateShellModeBadge as _updateShellModeBadge,
  updateShellModeDocumentTitle as _updateShellModeDocumentTitle,
  updateShellModeMasthead as _updateShellModeMasthead,
  renderShellModeGuide as _renderShellModeGuide,
  toggleShellModeEntryGrid as _toggleShellModeEntryGrid,
  toggleShellModeStatusChrome as _toggleShellModeStatusChrome,
  toggleAdminShellRoleVisibility as _toggleAdminShellRoleVisibility,
  updateShellEntryCards as _updateShellEntryCards,
  updatePanelTitles as _updatePanelTitles,
} from "./shell-mode-view.js";
import {
  workspaceStorageKey as _workspaceStorageKey,
  chatPaneStorageKey as _chatPaneStorageKey,
} from "./shell-storage-keys.js";
import {
  buildRoomVisualModel,
  renderPortraitCanvas,
  renderStageCanvas,
} from "./pretext-stage.js";
import {
  badgeToken as _badgeToken,
  defaultActiveRoomId as _defaultActiveRoomId,
  filteredRooms as _filteredRooms,
  initRail,
  latestRoomMessageLike as _latestRoomMessageLike,
  roomActivityTime as _roomActivityTime,
  roomDisplayPeer as _roomDisplayPeer,
  roomGroupBlueprints as _roomGroupBlueprints,
  roomKind as _roomKind,
  roomThreadHeadline as _roomThreadHeadline,
} from "./shell-room-rail.js";
import {
  roomFollowUpCountForState,
  roomChatStatusSummaryForState,
  roomQueueSummaryForState,
  roomSummaryLineForState,
  roomStatusLineForState,
  roomOwnershipForState,
  roomHostLabelForState,
} from "./shell-room-summary.js";
import {
  chatDetailRoomContextModelForState,
  directRoomPeerOnlineStatusForState,
  roomContextSummaryForState,
  roomRouteLabelForState,
  roomMemberCountForState,
  roomAudienceLabelForState,
} from "./shell-room-context.js";
import {
  isVisitorIdentity,
  residentScopedShellStatePage,
  translateClientDisplayName,
  translateRoutePrefix,
} from "./shell-identity.js";

const DEFAULT_BOOTSTRAP = {
  host: {
    client_profile: {
      class: "MobileWeb",
      display_name: "移动网页端",
      max_memory_kib: 8192,
      supports_graphics: true,
      supports_voice: true,
      supports_camera: false,
      supports_background_sync: false,
    },
    preferred_surface: "CompactTerminal",
    max_inline_chars: 512,
    supports_push_notifications: true,
    supports_voice_input: true,
    supports_camera_ingest: false,
    supports_background_sync: false,
  },
  shell: {
    route_prefix: "/app",
    supports_offline_shell: true,
    storage_mode: "IndexedDbPreferred",
    stream_incremental_updates: true,
  },
  initial_surface: "RoomList",
  offline_cache_budget_mb: 64,
  supports_background_resync: false,
};

const SAMPLE_STATE = {
  rooms: [
    {
      id: "dm:rsaga:builder",
      title: "私信 · 内测同伴",
      subtitle: "一对一测试聊天",
      meta: "最近 24 小时活跃",
      kind_hint: "私信",
      participant_label: "你与内测同伴",
      member_count: 2,
      scene_banner: "直接协作",
      scene_summary: "适合快速确认方向、补一句进度和直接追问。",
      caretaker: {
        name: "旺财",
        role_label: "房间管家",
        persona: "高冷但可靠，会替主人记住来访者和留言。",
        status: "在线值守",
        memory: "今天记录了 2 位访客、1 条重要留言。",
        auto_reply: "主人正在调试新版本，紧急事项我会先记录再提醒。",
        pending_visitors: 2,
        notifications: [
          "城东的李四问你是否还接移动端布局单。",
        ],
        messages: [
          {
            visitor: "城东的李四",
            note: "下午来过，看了看你的装备架，问你是否接移动端壳层单。",
            urgency: "普通",
          },
          {
            visitor: "南岸的阿梨",
            note: "留了一句“新群入口什么时候开”，没有继续追问。",
            urgency: "低",
          },
        ],
        patrol: {
          mode: "轻巡视",
          last_check: "2 分钟前",
          outcome: "最近一次内容巡视没有发现违规文本。",
        },
      },
      messages: [
        {
          sender: "内测同伴",
          timestamp: "10:14",
          text: "核心已经拆成无头聊天内核、宿主适配层和可选 AI 旁路。",
        },
        {
          sender: "builder",
          timestamp: "10:15",
          text: "住宅页先按这个像素房间方向走，别再加复杂工作台。",
        },
        {
          sender: "内测同伴",
          timestamp: "10:16",
          text: "先把 H5 跑起来，苹果和安卓就能更早接入。",
        },
        {
          sender: "builder",
          timestamp: "10:17",
          text: "对话保持微信那种左人右己，场景点击可以临时清屏。",
        },
        {
          sender: "内测同伴",
          timestamp: "10:18",
          text: "穿戴端先保持轻量，只做一眼卡片和语音回复，摄像头能力后置。",
        },
      ],
    },
    {
      id: "room:world:lobby",
      title: "群聊 · 世界广场",
      subtitle: "公开讨论与公告",
      meta: "公开群聊",
      kind_hint: "公开频道",
      participant_label: "跨城公开讨论",
      member_count: 12,
      scene_banner: "公共频道",
      scene_summary: "适合看公告、接运营通知和围观跨城公开讨论。",
      caretaker: {
        name: "巡逻犬",
        role_label: "频道巡视",
        persona: "只在需要时出来提醒，不抢公共讨论。",
        status: "低打扰巡视",
        memory: "记录最近 30 分钟的公共频道风险提示。",
        auto_reply: "如果内容触发巡视规则，会先投一张提示卡给城主。",
        pending_visitors: 0,
        notifications: [
          "刚完成一轮公共频道合法性巡视，当前没有新的违规告警。",
        ],
        messages: [],
        patrol: {
          mode: "合法性巡视",
          last_check: "6 分钟前",
          outcome: "仅提示 1 条外链待人工复核，没有拦截正常聊天。",
        },
      },
      messages: [
        {
          sender: "系统",
          timestamp: "09:40",
          text: "欢迎来到世界广场，这里像普通群聊一样显示公开讨论。",
        },
        {
          sender: "城民阿岚",
          timestamp: "09:42",
          text: "公告栏那边刚更新了今晚的联调说明，有空可以过去看一眼。",
        },
        {
          sender: "builder",
          timestamp: "09:43",
          text: "收到。主城先保持群聊主轴，热点只做入口提示，不抢画面。",
        },
        {
          sender: "巡逻犬",
          timestamp: "09:44",
          text: "当前频道正常，只有公告栏有一条待确认提醒。",
        },
      ],
    },
    {
      id: "room:world:design",
      title: "群聊 · 壳层讨论",
      subtitle: "移动端布局与视觉",
      meta: "讨论群",
      kind_hint: "工作群",
      participant_label: "产品与设计讨论",
      member_count: 6,
      scene_banner: "设计串场",
      scene_summary: "把布局、会话列表和输入区当成产品功能，而不是演示面板。",
      caretaker: {
        name: "灰狗",
        role_label: "房间小狗",
        persona: "暴躁修仙者，嘴硬但会把留言记清楚。",
        status: "半自动值守",
        memory: "帮主人盯着设计改动和访客留言。",
        auto_reply: "主人在画线框，急事留关键词，我会推送。",
        pending_visitors: 1,
        notifications: [
          "有人提到“治理入口太重”，已记成待处理提醒。",
        ],
        messages: [
          {
            visitor: "设计者",
            note: "提了一嘴：会话列表要更像正常 IM，不要像后台。",
            urgency: "高",
          },
        ],
        patrol: {
          mode: "房间巡逻",
          last_check: "刚刚",
          outcome: "已把高频反馈整理成给主人看的待办提醒。",
        },
      },
      messages: [
        {
          sender: "设计者",
          timestamp: "更早",
          text: "浏览器壳保持轻薄，但会话列表、消息流和输入框要像常见聊天产品。",
        },
      ],
    },
  ],
};

// A configured Gateway is authoritative. If its shell projection is
// unavailable, keep the H5 surface empty instead of presenting generated or
// cached demo rooms as if they were live server state.
const GATEWAY_EMPTY_STATE = Object.freeze({ rooms: [] });

function roomStageSummary(room) {
  return roomStageSummaryForState({
    room,
    stage: room ? stageProjection(room) : null,
    caretaker: room ? caretakerProfile(room) : null,
    contextSummary: room ? roomContextSummary(room) : "",
  });
}

function roomStagePortraitSummary(room) {
  return roomStagePortraitSummaryForState({
    room,
    portrait: room ? portraitProjection(room) : null,
    caretaker: room ? caretakerProfile(room) : null,
    contextSummary: room ? roomContextSummary(room) : "",
  });
}

function roomStagePortraitTitle(room) {
  return roomStagePortraitTitleForState({
    room,
    portrait: room ? portraitProjection(room) : null,
    caretaker: room ? caretakerProfile(room) : null,
  });
}
function roomStagePortraitChips(room) {
  const portrait = room ? portraitProjection(room) : null;
  return roomStagePortraitChipsForState({
    room,
    portrait,
    caretaker: room ? caretakerProfile(room) : null,
    badgeText: room
      ? portrait?.badge || room.scene_banner || translateRoomKindForShellPage(roomKind(room), "user")
      : "",
    audienceLabel: room ? roomAudienceLabel(room) : "",
    memberCount: room ? roomMemberCount(room) : 0,
    pendingCount: room ? caretakerPendingCount(room) : 0,
  });
}

function appendRoomQuickStateAdvanceButton(actions, room, options = {}) {
  const action = latestRoomQuickAction(room);
  const currentState = latestRoomQuickState(room);
  const secondarySpec = inlineActionProfile(room, "secondary");
  const nextAction = secondarySpec?.action || action;
  const nextState = secondarySpec?.next_state || "";
  const label = secondarySpec?.label || quickActionAdvanceLabel(action, currentState);
  if (!actions || !label || !room?.id) return;
  const button = document.createElement("button");
  button.type = "button";
  if (options.className) {
    button.className = options.className;
  }
  if (options.dataset) {
    Object.assign(button.dataset, options.dataset);
  }
  button.textContent = label;
  button.addEventListener("click", () => {
    if (nextState) {
      setRoomQuickAction(room.id, nextAction);
      setRoomQuickState(room.id, nextAction, nextState);
      renderRooms();
      renderTimeline();
      renderConversationOverview();
      renderChatDetailPanel();
      return;
    }
    advanceRoomQuickState(room.id);
  });
  actions.appendChild(button);
}

function syncUserQuickActionButtons(roomId = activeRoomId) {
  if (!chatDetailCardActionsEl) return;
  const activeAction = roomQuickAction(roomId);
  for (const button of chatDetailCardActionsEl.querySelectorAll("[data-card-action]")) {
    const matches = button.dataset.cardAction === activeAction;
    button.classList.toggle("is-active", matches);
    button.setAttribute("aria-pressed", matches ? "true" : "false");
  }
}

function userRoomProjection(room, visual) {
  return userRoomProjectionForState({
    room,
    visual,
    fallback: shellModeConfig("user"),
    detailCard: room ? detailCardProfile(room) : null,
    caretaker: room ? caretakerProfile(room) : null,
  });
}

function sceneImageLayerEnv() {
  return {
    timeOfDay: globalThis.document?.body?.dataset?.timeOfDay,
    matchMedia: (q) => globalThis.window?.matchMedia?.(q),
  };
}

function presetImageLayerUrl(preset) {
  return presetImageLayerUrlForState(preset, sceneImageLayerEnv());
}

function timeAdjustedRuntimeSceneUrl(candidate) {
  return timeAdjustedRuntimeSceneUrlForState(candidate, sceneImageLayerEnv());
}

function imageLayerUrl(imageLayer) {
  return imageLayerUrlForState(imageLayer, sceneImageLayerEnv());
}

function applyUserSceneImageLayer(room) {
  if (currentShellPage() !== "user") return;
  const stage = document.querySelector(".creative-stage, .user-stage");
  if (!stage) return;
  const url = imageLayerUrl(room?.image_layer);
  if (!url) {
    if (typeof stage.style.removeProperty === "function") {
      stage.style.removeProperty("--creative-scene-image");
    } else {
      stage.style["--creative-scene-image"] = "";
    }
    stage.removeAttribute("data-image-layer-id");
    stage.removeAttribute("data-image-layer-preset");
    return;
  }
  if (typeof stage.style.setProperty === "function") {
    stage.style.setProperty("--creative-scene-image", `url(${JSON.stringify(url)})`);
  } else {
    stage.style["--creative-scene-image"] = `url(${JSON.stringify(url)})`;
  }
  if (room?.image_layer?.layer_id) stage.dataset.imageLayerId = room.image_layer.layer_id;
  else stage.removeAttribute("data-image-layer-id");
  if (room?.image_layer?.preset) stage.dataset.imageLayerPreset = room.image_layer.preset;
  else stage.removeAttribute("data-image-layer-preset");
}

function syncUserRoomProjection(room, visual) {
  if (currentShellPage() !== "user") return;
  const projection = userRoomProjection(room, visual);
  applyUserSceneImageLayer(room);

  // 标记房间所有权：自己的私宅 vs 访客模式
  // room.owner_resident_id 由 gateway 暴露（personal_room 的主人；双方DM/公共为 None）。
  // owner===identity → own；owner≠identity → visitor；None → 不显示。
  const identity = currentIdentity();
  const ownership = roomOwnershipForState(room, identity);
  setDatasetFlag(document.body, "roomOwnership", ownership);
  if (ownership === "visitor") {
    const hostLabel = roomHostLabelForState(room) || "对方";
    if (hudOwnershipVisitorBadgeEl) {
      hudOwnershipVisitorBadgeEl.textContent = `你在 ${hostLabel} 的私宅中`;
    }
  } else if (ownership === "own" && hudOwnershipOwnBadgeEl) {
    hudOwnershipOwnBadgeEl.textContent = "我的私宅";
  }

  setDatasetFlag(document.body, "roomVariant", projection.variant);
  setDatasetFlag(document.body, "roomMotif", projection.motif);
  setDatasetFlag(appShellEl, "roomVariant", projection.variant);
  setDatasetFlag(appShellEl, "roomMotif", projection.motif);
  setDatasetFlag(roomsPanelEl, "roomVariant", projection.variant);
  setDatasetFlag(roomsPanelEl, "roomMotif", projection.motif);
  setDatasetFlag(conversationPanelEl, "roomVariant", projection.variant);
  setDatasetFlag(conversationPanelEl, "roomMotif", projection.motif);
  setDatasetFlag(chatDetailPanelEl, "roomVariant", projection.variant);
  setDatasetFlag(chatDetailPanelEl, "roomMotif", projection.motif);
  setDatasetFlag(roomStageSideEl, "roomVariant", projection.variant);
  setDatasetFlag(roomStageSideEl, "roomMotif", projection.motif);

  if (mastheadEyebrowEl) {
    mastheadEyebrowEl.textContent = projection.eyebrow;
  }
  if (mastheadTitleEl) {
    mastheadTitleEl.textContent = projection.title;
  }
  if (heroNoteEl) {
    heroNoteEl.textContent = projection.hero;
  }
  if (chatDetailSummaryTitleEl) {
    chatDetailSummaryTitleEl.textContent = projection.detailTitle;
  }
  if (chatDetailSummaryCopyEl) {
    chatDetailSummaryCopyEl.textContent = projection.detailCopy;
  }
  syncUserDetailCard(room, visual, projection);
}

function userDetailCardProjection(room, visual, projection) {
  return userDetailCardProjectionForState(room, visual, projection, {
    roomChatStatusSummary,
    currentIdentity,
    roomDisplayPeer,
    roomAudienceLabel,
  });
}


function applyUserDetailCardShellState(card) {
  setDatasetFlag(chatDetailCardShellEl, "roomVariant", card.variant);
  setDatasetFlag(chatDetailCardShellEl, "roomMotif", card.motif);
  setDatasetFlag(chatDetailCardActionsEl, "roomVariant", card.variant);
  setDatasetFlag(chatDetailCardActionsEl, "roomMotif", card.motif);
  setDatasetFlag(chatDetailCardAvatarEl, "roomVariant", card.variant);
  setDatasetFlag(chatDetailCardAvatarEl, "monogram", card.monogram);

  if (chatDetailCardKickerEl) {
    chatDetailCardKickerEl.textContent = card.kicker;
  }
  if (chatDetailCardTitleEl) {
    chatDetailCardTitleEl.textContent = card.title;
  }
  if (chatDetailCardAvatarEl) {
    chatDetailCardAvatarEl.textContent = card.monogram;
  }
  if (chatDetailCardMetaEl) {
    clearChildren(chatDetailCardMetaEl);
    for (const item of card.meta) {
      chatDetailCardMetaEl.appendChild(createChatDetailCardMetaRow(item.label, item.value));
    }
  }
}

function clearUserDetailCardTransientNodes() {
  if (!chatDetailCardShellEl) return;
  for (const node of Array.from(chatDetailCardShellEl.querySelectorAll(".chat-detail-card-workflow"))) {
    node.remove();
  }
  for (const node of Array.from(chatDetailCardShellEl.querySelectorAll(".chat-detail-card-preview"))) {
    node.remove();
  }
}

function insertUserDetailCardTransientNode(node) {
  if (!node || !chatDetailCardShellEl) return;
  if (chatDetailCardActionsEl?.parentNode === chatDetailCardShellEl) {
    chatDetailCardShellEl.insertBefore(node, chatDetailCardActionsEl);
    return;
  }
  chatDetailCardShellEl.appendChild(node);
}

function createUserDetailCardWorkflowNode(room, quickAction, quickState) {
  return createWorkflowProgress(quickAction, quickState, {
    className: "chat-detail-card-workflow",
    title: quickAction ? `${quickAction}阶段` : "",
    stages: workflowProfile(room)?.steps,
    onStageClick: (stage) => {
      previewRoomQuickStage(room?.id || activeRoomId, quickAction, stage.label);
      seedComposerFromQuickAction(quickAction, quickActionWorkflowTemplate(quickAction, stage.label), { force: true });
    },
  });
}

function createUserDetailCardPreviewNode(room, quickAction, preview) {
  const previewState = preview?.state || "";
  const previewSnapshotIndex = preview?.snapshotIndex ?? null;
  const previewStructured = preview?.structured || null;
  return createQuickActionPreviewCard(quickAction, previewState, previewStructured, {
    className: "chat-detail-card-preview",
    maxFields: 2,
    roomId: room?.id || activeRoomId,
    historyLabel: preview?.historyLabel || "",
    fieldView: roomQuickPreviewCardFieldView(
      room?.id || activeRoomId,
      quickAction,
      previewState,
      previewSnapshotIndex,
    ),
    history: preview?.history || [],
    selectedHistoryIndex: previewSnapshotIndex,
    onHistoryClick: (_snapshot, index) => {
      previewRoomQuickStage(room?.id || activeRoomId, quickAction, previewState, index);
    },
    onFieldViewChange: (viewId) => {
      setRoomQuickPreviewCardFieldView(
        room?.id || activeRoomId,
        quickAction,
        previewState,
        previewSnapshotIndex,
        viewId,
      );
    },
  });
}

function renderUserDetailCardDynamicSections(room, quickAction, quickState, preview) {
  if (!chatDetailCardShellEl) return;
  clearUserDetailCardTransientNodes();
  const workflow = createUserDetailCardWorkflowNode(room, quickAction, quickState);
  insertUserDetailCardTransientNode(workflow);
  const previewCard = createUserDetailCardPreviewNode(room, quickAction, preview);
  insertUserDetailCardTransientNode(previewCard);
}

function createUserDetailCardActionButton(action) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "chat-detail-card-action";
  button.dataset.cardAction = action;
  button.setAttribute("aria-pressed", "false");
  button.textContent = action;
  button.addEventListener("click", () => {
    seedComposerFromQuickAction(action);
  });
  return button;
}

function renderUserDetailCardActions(room, card) {
  if (!chatDetailCardActionsEl) return;
  clearChildren(chatDetailCardActionsEl);
  for (const action of card.actions) {
    chatDetailCardActionsEl.appendChild(createUserDetailCardActionButton(action));
  }
  appendRoomQuickActionOverviewButton(chatDetailCardActionsEl, room, {
    className: "chat-detail-card-action chat-detail-card-action-workflow",
    dataset: { cardWorkflowAction: "true" },
  });
  appendRoomQuickStateAdvanceButton(chatDetailCardActionsEl, room, {
    className: "chat-detail-card-action chat-detail-card-action-advance",
    dataset: { cardStateAdvance: "true" },
  });
  syncUserQuickActionButtons(room?.id || activeRoomId);
}

function syncUserDetailCard(room, visual, projection) {
  if (currentShellPage() !== "user") return;
  const card = userDetailCardProjection(room, visual, projection);
  const quickAction = latestRoomQuickAction(room);
  const quickState = latestRoomQuickState(room);
  const preview = resolveRoomQuickPreview(room, quickAction);
  applyUserDetailCardShellState(card);
  renderUserDetailCardDynamicSections(room, quickAction, quickState, preview);
  renderUserDetailCardActions(room, card);
}


function ensureUserSceneChrome() {
  if (currentShellPage() !== "user") return;
  ensureRoomStageSideChrome();
  ensureRoomStagePortraitCanvasChrome();
  ensureRoomStageCanvasChrome();
  ensureChatDetailPanelChrome();
}

function ensureRoomStageSideChrome() {
  if (!conversationStageEl || (roomStageSideEl && roomStageSideEl.isConnected)) return;
  roomStageSideEl = createRoomStageSideElement();
  insertRoomStageSideElement();
}

function createRoomStageSideElement() {
  return _createRoomStageSideElement();
}

function insertRoomStageSideElement() {
  const sideAnchor = conversationStageCopyEl || conversationStageEl.firstChild || null;
  if (sideAnchor && sideAnchor.parentNode === conversationStageEl) {
    sideAnchor.insertAdjacentElement("afterend", roomStageSideEl);
  } else {
    conversationStageEl.appendChild(roomStageSideEl);
  }
}

function createRoomStageCanvasChrome(id, label) {
  return _createRoomStageCanvasChrome(id, label);
}

function ensureRoomStagePortraitCanvasChrome() {
  if (!roomStageSideEl || (roomStagePortraitCanvasEl && roomStagePortraitCanvasEl.isConnected)) return;
  const chrome = createRoomStageCanvasChrome("room-stage-portrait-canvas", "房间角色资料画布");
  roomStagePortraitCanvasWrapEl = chrome.wrap;
  roomStagePortraitCanvasEl = chrome.canvas;
  roomStageSideEl.appendChild(roomStagePortraitCanvasWrapEl);
}

function ensureRoomStageCanvasChrome() {
  if (!conversationStageCopyEl || (roomStageCanvasEl && roomStageCanvasEl.isConnected)) return;
  const chrome = createRoomStageCanvasChrome("room-stage-canvas", "房间场景文字画布");
  roomStageCanvasWrapEl = chrome.wrap;
  roomStageCanvasEl = chrome.canvas;
  insertRoomStageCanvasWrap();
}

function insertRoomStageCanvasWrap() {
  const noteAnchor = roomStageNoteEl?.isConnected ? roomStageNoteEl : null;
  if (noteAnchor) {
    noteAnchor.insertAdjacentElement("beforebegin", roomStageCanvasWrapEl);
  } else {
    conversationStageCopyEl.appendChild(roomStageCanvasWrapEl);
  }
}

function ensureChatDetailPanelChrome() {
  if (!chatDetailPanelEl || !chatDetailPanelEl.isConnected) {
    chatDetailPanelEl = createChatDetailPanelChrome();
    insertChatDetailPanelChrome();
    return;
  }
  showChatDetailPanelChrome();
}

function createChatDetailPanelChrome() {
  const result = _createChatDetailPanelChrome();
  chatDetailContentEl = result.contentEl;
  return result.panel;
}

function insertChatDetailPanelChrome() {
  if (conversationPanelEl?.parentNode === layoutEl) {
    conversationPanelEl.insertAdjacentElement("afterend", chatDetailPanelEl);
  } else {
    layoutEl?.appendChild(chatDetailPanelEl);
  }
}

function showChatDetailPanelChrome() {
  setInlineStyle(chatDetailPanelEl, "display", "block", true);
  setInlineStyle(chatDetailPanelEl, "grid-column", "1 / -1", true);
}

function renderRoomStagePortrait(room) {
  if (!roomStageSideEl) return;
  const visual = buildRoomVisualModel(
    room,
    roomStageSummary(room),
    {
      title: roomStagePortraitTitle(room),
      summary: roomStagePortraitSummary(room),
    },
  );
  _renderRoomStagePortraitChrome(
    {
      sideEl: roomStageSideEl,
      canvasWrapEl: roomStagePortraitCanvasWrapEl,
      canvasEl: roomStagePortraitCanvasEl,
      portrait: visual.portrait,
      chips: roomStagePortraitChips(room),
    },
    {
      createChip: createStageChip,
      renderPortrait: renderPortraitCanvas,
    },
  );
}

const sidebarStackEl = document.querySelector(".sidebar-stack");
let caretakerPanelEl = null;
let caretakerStatusEl = null;

const roomListEl = document.querySelector("#room-list");
const timelineEl = document.querySelector("#timeline");
const metaEl = document.querySelector("#conversation-meta");
const roomStageTitleEl = document.querySelector("#room-stage-title");
const conversationStageEl = document.querySelector(".conversation-stage");
const conversationStageCopyEl = document.querySelector(".conversation-stage-copy");
let roomStageCanvasEl = document.querySelector("#room-stage-canvas");
let roomStageCanvasWrapEl = roomStageCanvasEl?.closest(".conversation-stage-canvas-wrap") || null;
let roomStageNoteEl =
  document.querySelector("#room-stage-note") || document.querySelector(".conversation-stage-note");
let roomStageSideEl = document.querySelector(".conversation-stage-side");
let roomStagePortraitCanvasEl = document.querySelector("#room-stage-portrait-canvas");
let roomStagePortraitCanvasWrapEl =
  roomStagePortraitCanvasEl?.closest(".conversation-stage-canvas-wrap") || null;
const transportStateEl = document.querySelector("#transport-state");
const storageStateEl = document.querySelector("#storage-state");
const gatewayStateEl = document.querySelector("#gateway-state");
const providerStateEl = document.querySelector("#provider-state");
const worldStateEl = document.querySelector("#world-state");
const shellModeBadgeEl = document.querySelector("#shell-mode-badge");
const shellEntryCards = Array.from(document.querySelectorAll("[data-shell-entry]"));
const mastheadEyebrowEl = document.querySelector("#masthead-eyebrow");
const mastheadTitleEl = document.querySelector("#masthead-title");
const heroNoteEl = document.querySelector("#hero-note");
const entryGridEl = document.querySelector("#entry-grid");
const modeGuideEl = document.querySelector("#mode-guide");
const worldSummaryEl = document.querySelector("#world-summary");
const governanceStatusEl = document.querySelector("#governance-status");
const worldDirectoryListEl = document.querySelector("#world-directory-list");
const worldMirrorSourceListEl = document.querySelector("#world-mirror-source-list");
const worldSquareListEl = document.querySelector("#world-square-list");
const worldSafetyListEl = document.querySelector("#world-safety-list");
const providerConnectFormEl = document.querySelector("#provider-connect-form");
const providerUrlInputEl = document.querySelector("#provider-url-input");
const providerDisconnectButtonEl = document.querySelector("#provider-disconnect-button");
const authStatusEl = document.querySelector("#auth-status");
const authRequestFormEl = document.querySelector("#auth-request-form");
const authDeliverySelectEl = document.querySelector("#auth-delivery-select");
const authResidentInputEl = document.querySelector("#auth-resident-input");
const authNicknameInputEl = document.querySelector("#auth-nickname-input");
const authNicknameEditorEl = document.querySelector("#auth-nickname-editor");
const authNicknameEditInputEl = document.querySelector("#auth-nickname-edit-input");
const authNicknameSaveBtnEl = document.querySelector("#auth-nickname-save-btn");
const authEmailInputEl = document.querySelector("#auth-email-input");
const authMobileInputEl = document.querySelector("#auth-mobile-input");
const authDeviceInputEl = document.querySelector("#auth-device-input");
const authVerifyFormEl = document.querySelector("#auth-verify-form");
const authChallengeInputEl = document.querySelector("#auth-challenge-input");
const authCodeInputEl = document.querySelector("#auth-code-input");
const residentLoginCardEl = document.querySelector("#resident-login-card");
const residentLoginOverlayEl = document.querySelector("#resident-login-overlay");
const residentLoginCloseEl = document.querySelector("#resident-login-close");
const hudLoginToggleEl = document.querySelector("#hud-login-toggle");
const hudOwnershipOwnBadgeEl = document.querySelector(".creative-hud-ownership-badge--own");
const hudOwnershipVisitorBadgeEl = document.querySelector(".creative-hud-ownership-badge--visitor");
const personalRoomPolicyControlEl = document.querySelector("#personal-room-access-policy");
const personalRoomPolicyStatusEl = document.querySelector("#personal-room-access-policy-status");
const personalRoomPolicyButtons = Array.from(document.querySelectorAll("[data-personal-room-policy]"));
const cityListEl = document.querySelector("#city-list");
const residentListEl = document.querySelector("#resident-list");
const exportFormatSelectEl = document.querySelector("#export-format-select");
const exportCurrentButtonEl = document.querySelector("#export-current-button");
const exportAllButtonEl = document.querySelector("#export-all-button");
const composerFormEl = document.querySelector("#composer");
const composerInputEl = document.querySelector("#composer-input");
const composerSendEl = document.querySelector("#composer-send");
const composerMentionTriggerEl = document.querySelector("[data-mention-trigger]");
const composerSymbolTriggerEl = document.querySelector("[data-symbol-trigger]");
const composerSymbolMenuEl = document.querySelector("[data-symbol-menu]");
const composerSymbolInsertEls = Array.from(document.querySelectorAll("[data-symbol-insert]"));
const identityInputEl = document.querySelector("#identity-input");
const cityCreateFormEl = document.querySelector("#city-create-form");
const cityJoinFormEl = document.querySelector("#city-join-form");
const roomCreateFormEl = document.querySelector("#room-create-form");
const cityTitleInputEl = document.querySelector("#city-title-input");
const citySlugInputEl = document.querySelector("#city-slug-input");
const cityDescriptionInputEl = document.querySelector("#city-description-input");
const cityJoinInputEl = document.querySelector("#city-join-input");
const roomCityInputEl = document.querySelector("#room-city-input");
const roomTitleInputEl = document.querySelector("#room-title-input");
const roomSlugInputEl = document.querySelector("#room-slug-input");
const roomDescriptionInputEl = document.querySelector("#room-description-input");
const directOpenFormEl = document.querySelector("#direct-open-form");
const directPeerInputEl = document.querySelector("#direct-peer-input");
const worldMirrorFormEl = document.querySelector("#world-mirror-form");
const worldMirrorUrlInputEl = document.querySelector("#world-mirror-url-input");
const worldNoticeFormEl = document.querySelector("#world-notice-form");
const worldNoticeTitleInputEl = document.querySelector("#world-notice-title-input");
const worldNoticeSeveritySelectEl = document.querySelector("#world-notice-severity-select");
const worldNoticeTagsInputEl = document.querySelector("#world-notice-tags-input");
const worldNoticeBodyInputEl = document.querySelector("#world-notice-body-input");
const worldTrustFormEl = document.querySelector("#world-trust-form");
const worldTrustCityInputEl = document.querySelector("#world-trust-city-input");
const worldTrustStateSelectEl = document.querySelector("#world-trust-state-select");
const worldTrustReasonInputEl = document.querySelector("#world-trust-reason-input");
const worldAdvisoryFormEl = document.querySelector("#world-advisory-form");
const worldAdvisorySubjectKindSelectEl = document.querySelector(
  "#world-advisory-subject-kind-select",
);
const worldAdvisorySubjectInputEl = document.querySelector("#world-advisory-subject-input");
const worldAdvisoryActionInputEl = document.querySelector("#world-advisory-action-input");
const worldAdvisoryReasonInputEl = document.querySelector("#world-advisory-reason-input");
const worldReportReviewFormEl = document.querySelector("#world-report-review-form");
const worldReportReviewIdInputEl = document.querySelector("#world-report-review-id-input");
const worldReportReviewStatusSelectEl = document.querySelector(
  "#world-report-review-status-select",
);
const worldReportReviewCityStateSelectEl = document.querySelector(
  "#world-report-review-city-state-select",
);
const worldReportReviewResolutionInputEl = document.querySelector(
  "#world-report-review-resolution-input",
);
const worldReportFormEl = document.querySelector("#world-report-form");
const worldReportCityInputEl = document.querySelector("#world-report-city-input");
const worldReportTargetKindSelectEl = document.querySelector("#world-report-target-kind-select");
const worldReportTargetInputEl = document.querySelector("#world-report-target-input");
const worldReportSummaryInputEl = document.querySelector("#world-report-summary-input");
const worldReportEvidenceInputEl = document.querySelector("#world-report-evidence-input");
const worldResidentSanctionFormEl = document.querySelector("#world-resident-sanction-form");
const worldResidentIdInputEl = document.querySelector("#world-resident-id-input");
const worldResidentCityInputEl = document.querySelector("#world-resident-city-input");
const worldResidentEmailInputEl = document.querySelector("#world-resident-email-input");
const worldResidentMobileInputEl = document.querySelector("#world-resident-mobile-input");
const worldResidentDeviceInputEl = document.querySelector("#world-resident-device-input");
const worldResidentReasonInputEl = document.querySelector("#world-resident-reason-input");
const appShellEl = document.querySelector(".app");
const topbarEl = document.querySelector(".topbar");
const layoutEl = document.querySelector(".layout");
const guidePanelEl = document.querySelector(".guide-panel");
const governancePanelEl = document.querySelector(".governance");
const authPanelEl = document.querySelector(".auth");
const roomsPanelEl = document.querySelector(".rooms");
const conversationPanelEl = document.querySelector(".conversation");
let chatDetailPanelEl = document.querySelector(".chat-detail");
let chatDetailContentEl = document.querySelector("#chat-detail-content");
let chatDetailSummaryTitleEl =
  document.querySelector("#chat-detail-summary-title") || document.querySelector(".chat-detail-summary-title");
let chatDetailSummaryCopyEl =
  document.querySelector("#chat-detail-summary-copy") || document.querySelector(".chat-detail-summary-copy");
let chatDetailCardShellEl = document.querySelector("#chat-detail-card-shell");
let chatDetailCardKickerEl =
  document.querySelector("#chat-detail-card-kicker") || document.querySelector(".chat-detail-card-kicker");
let chatDetailCardTitleEl =
  document.querySelector("#chat-detail-card-title") || document.querySelector(".chat-detail-card-title");
let chatDetailCardAvatarEl =
  document.querySelector("#chat-detail-card-avatar") || document.querySelector(".chat-detail-card-avatar");
let chatDetailCardMetaEl =
  document.querySelector("#chat-detail-card-meta") || document.querySelector(".chat-detail-card-meta");
let chatDetailCardActionsEl =
  document.querySelector("#chat-detail-card-actions") || document.querySelector(".chat-detail-card-actions");
const guidePanelTitleEl = guidePanelEl?.querySelector(".panel-title");
const governancePanelTitleEl = governancePanelEl?.querySelector(".panel-title");
const authPanelTitleEl = authPanelEl?.querySelector(".panel-title");
const roomsPanelTitleEl = roomsPanelEl?.querySelector(".panel-title");
const conversationPanelTitleEl = conversationPanelEl?.querySelector(".panel-title");

const governanceBrowseBlocks = [
  worldDirectoryListEl?.closest(".governance-block"),
  worldMirrorSourceListEl?.closest(".governance-block"),
  worldSquareListEl?.closest(".governance-block"),
  worldSafetyListEl?.closest(".governance-block"),
  cityListEl?.closest(".governance-block"),
  residentListEl?.closest(".governance-block"),
].filter(Boolean);

const worldActionForms = [cityJoinFormEl, directOpenFormEl, worldReportFormEl].filter(Boolean);
const governanceAdminForms = [
  providerConnectFormEl,
  cityCreateFormEl,
  roomCreateFormEl,
  worldMirrorFormEl,
  worldNoticeFormEl,
  worldTrustFormEl,
  worldAdvisoryFormEl,
  worldReportReviewFormEl,
  worldResidentSanctionFormEl,
].filter(Boolean);

let messageSearchController = null;
const { searchBar } = mountMessageSearchChrome(
  {
    timelineEl,
    stageSideEl: roomStageSideEl,
    onToggle: () => messageSearchController?.toggle(),
  },
);

let bootstrap = DEFAULT_BOOTSTRAP;
let state = structuredClone(SAMPLE_STATE);
setStateGetter(() => state);
let shellMode = "unified";
let governance = {
  world: null,
  portability: null,
  cities: [],
  memberships: [],
  public_rooms: [],
  residents: [],
  world_directory: null,
  world_mirror_sources: [],
  world_square: [],
  world_safety: null,
};
let activeRoomId = defaultActiveRoomId(state.rooms);
let gatewayUrl = null;
const worldSurfaceRenderers = createWorldSurfaceRenderers({
  worldDirectoryListEl,
  worldMirrorSourceListEl,
  worldSquareListEl,
  worldSafetyListEl,
  getGatewayUrl: () => gatewayUrl,
  getWorldDirectory: () => governance.world_directory,
  getWorldMirrorSources: () => governance.world_mirror_sources,
  getWorldSquare: () => governance.world_square,
  getWorldSafety: () => governance.world_safety,
});
const governanceCitySurfaceRenderer = createGovernanceCitySurfaceRenderer({
  cityListEl,
  worldStateEl,
  worldSummaryEl,
  worldDirectoryListEl,
  worldMirrorSourceListEl,
  worldSquareListEl,
  worldSafetyListEl,
  cityJoinInputEl,
  roomCityInputEl,
  roomTitleInputEl,
  focusRoom,
  loadGatewayState,
  renderRooms,
  renderTimeline,
  submitFreezeRoom,
  submitApproveResident,
  submitJoinCity,
  submitStewardUpdate,
  submitFederationPolicy,
  setGovernanceStatus,
});
const residentSurfaceRenderer = createResidentSurfaceRenderer({
  residentListEl,
  getGatewayUrl: () => gatewayUrl,
  getResidents: () => governance.residents,
  getIdentity: currentIdentity,
  getShellPage: currentShellPage,
  getSearchModeControls: () => searchModeControlsEl,
  getSearchMode: () => searchMode,
  getRoomSearch: () => roomSearch,
  translateResidentLabelFn: translateResidentLabel,
  applyAvatarStyleFn: applyAvatarStyle,
  enterResidentRoom,
  setGovernanceStatus,
  postGatewayJson,
  refreshFromGateway,
});
messageSearchController = createMessageSearchController({
  doc: document,
  searchBar,
  getGatewayUrl: () => gatewayUrl,
  getRoomId: () => activeRoomId,
  getResidentId: () => currentIdentity(),
  getSessionToken: () => getSessionToken(),
});
messageSearchController.bind();
let lastShellStateVersion = null;
let senderIdentity = "访客";
let currentWorkspace = "chat";
let roomSearch = "";
let roomFilter = "all";
let searchMode = "all"; // "all" | "rooms" | "residents"
let chatPaneMode = "split";
let roomReadMarkers = {};
let roomDrafts = {};
let roomSendErrors = {};
const pendingEchoStore = createPendingMessageEchoStore({ getIdentity: currentIdentity });
let roomQuickStates = {};
let roomQuickStatePreviews = {};
let roomQuickSnapshots = {};
let personalRoomAccessPolicySaving = false;
let editingMessageTarget = null;
let followTimelineToLatest = false;
let residentLoginDismissed = false;
let provider = {
  mode: "unknown",
  base_url: null,
  connection_state: "Disconnected",
  reachable: false,
};
let providerLoaded = false;
let gatewayShellStateAvailable = false;
let workspaceNavEl = null;
let workspaceTabs = [];
let roomSearchInputEl = document.querySelector("#room-search-input");
let roomToolbarNoteEl = null;
let roomFilterButtons = [];
let searchModeControlsEl = null;
let searchModeSegments = [];
let conversationOverviewEl = null;
let conversationCalloutEl = null;
let modeBannerEl = null;
let governanceBriefEl = null;
let roomViewToggleButtonEl = null;
let roomDigestEl = null;
let threadStatusRailEl = null;
let composerStatusEl = null;
let composerHeroEl = null;
let composerContextEl = null;
let composerTipEl = null;
let composerMetaEl = null;
let lastSentMessage = "";
let lastComposerKeyboardSubmitAt = 0;
const threadStatusSurfaceRenderer = createThreadStatusSurfaceRenderer({
  doc: document,
  getRailEl: () => threadStatusRailEl,
  getModel: (room) => threadStatusRailModel(room, currentShellPage()),
  createLineFn: createLine,
  clearChildrenFn: clearChildren,
});
const roomDigestSurfaceRenderer = createRoomDigestSurfaceRenderer({
  doc: document,
  getRoomDigestEl: () => roomDigestEl,
  getRooms: () => state.rooms,
  getActiveRoomId: () => activeRoomId,
  getShellPage: currentShellPage,
  roomKindFn: roomKind,
  unreadCountFn: unreadCount,
  roomHasDraftFn: roomHasDraft,
  roomFollowUpCountFn: roomFollowUpCount,
  caretakerPendingCountFn: caretakerPendingCount,
  caretakerNotificationCountFn: caretakerNotificationCount,
  roomThreadHeadlineFn: roomThreadHeadline,
  roomContextSummaryFn: roomContextSummary,
  roomChatStatusSummaryFn: roomChatStatusSummary,
  roomQueueSummaryFn: roomQueueSummary,
  getRoomSendErrors: () => roomSendErrors,
  pendingEchoesForRoomFn: (roomId) => pendingEchoesForRoom(roomId),
  caretakerProfileFn: caretakerProfile,
  createPillFn: createPill,
  clearChildrenFn: clearChildren,
});
const roomListSurfaceRenderer = createRoomListSurfaceRenderer({
  roomListEl,
  getRoomToolbarNoteEl: () => roomToolbarNoteEl,
  getSearchModeControls: () => searchModeControlsEl,
  getSearchMode: () => searchMode,
  getAllRooms: () => state.rooms,
  getActiveRoomId: () => activeRoomId,
  getRoomFilter: () => roomFilter,
  getRoomSearch: () => roomSearch,
  getGatewayUrl: () => gatewayUrl,
  getRoomSendErrors: () => roomSendErrors,
  getShellPage: currentShellPage,
  getFilteredRooms: _filteredRooms,
  getRoomGroups: (shellPage, rooms) =>
    _roomGroupBlueprints(shellPage, rooms, activeRoomId, roomSendErrors, roomHasDraft, unreadCount),
  getRoomSyncLabel: roomSyncLabel,
  translateRoomKindFn: translateRoomKind,
  translateRoomKindForShellPageFn: translateRoomKindForShellPage,
  roomKindFn: roomKind,
  roomAudienceLabelFn: roomAudienceLabel,
  roomThreadHeadlineFn: roomThreadHeadline,
  directRoomPeerOnlineStatusFn: directRoomPeerOnlineStatus,
  confirmResidentRoomJumpFn: confirmResidentRoomJump,
  applyAvatarStyleFn: applyAvatarStyle,
  roomHasDraftFn: roomHasDraft,
  unreadCountFn: unreadCount,
  visiblePendingEchoCountFn: visiblePendingEchoCount,
  caretakerProfileFn: caretakerProfile,
  caretakerPendingCountFn: caretakerPendingCount,
  createRoomQuickActionPillFn: createRoomQuickActionPill,
  createRoomQuickPreviewPillFn: createRoomQuickPreviewPill,
  createRoomPreviewNodeFn: createRoomPreviewNode,
  createRoomInlineActionsFn: createRoomInlineActions,
  roomSummaryLineFn: roomSummaryLine,
  roomStatusLineFn: roomStatusLine,
  focusRoomFn: focusRoom,
  renderRoomsFn: renderRooms,
  renderTimelineFn: renderTimeline,
  renderRoomDigestFn: renderRoomDigest,
  ensureRoomQuickActionsFn: ensureRoomQuickActions,
});

const chatFocusController = createChatFocusController({
  doc: document,
  layoutEl,
  conversationPanelEl,
  getAnchor: () => roomViewToggleButtonEl,
  getWorkspace: () => currentWorkspace,
  loadPreference: loadChatFocusPreferenceFromState,
  persistPreference: persistChatFocusPreferenceInState,
  onStateApplied: updateChatPriorityBadgeText,
});

const gatewaySyncController = createGatewaySyncController({
  getGatewayUrl: () => gatewayUrl,
  loadWorldState,
  loadShellState: loadGatewayState,
  loadProviderState,
  formatError: localizedRuntimeError,
  onRefreshStart: () => {
    updateComposerState();
    renderConversationOverview();
  },
  onRefreshSettled: ({ worldChanged }) => {
    if (worldChanged) {
      if (!userShellProjection()) {
        renderGovernance();
      }
      renderResidents();
    }
    renderRooms();
    renderTimeline();
    refreshGatewayBadge();
    updateComposerState();
    updateAuthFormState();
    updateResidentLoginSurface();
    applyRailVisibility();
    syncPersonalRoomAccessPolicyControl();
    if (!userShellProjection()) {
      updateGovernanceFormState();
    }
  },
});

const gatewayPollingController = createGatewayPollingController({
  getGatewayUrl: () => gatewayUrl,
  getRefreshIntervalMs: () => bootstrap.refresh_interval_ms || 4000,
  isRefreshInProgress: gatewaySyncController.isRefreshing,
  isDocumentHidden: () => document.visibilityState === "hidden",
  refreshFromGateway,
  onPollingError: (error) => {
    gatewaySyncController.recordFailure(error, "同步失败");
    renderShellStateRefresh();
  },
  onForegroundError: (error, reason) => {
    console.warn(`[lobster-web-shell] foreground refresh failed (${reason})`, error);
  },
});

const gatewayRealtimeController = createGatewayRealtimeController({
  getGatewayUrl: () => gatewayUrl,
  getLastStateVersion: () => lastShellStateVersion,
  setLastStateVersion: (value) => { lastShellStateVersion = value; },
  buildEventsUrl: gatewayShellEventsUrl,
  applyShellStatePayload: applyGatewayShellStatePayload,
  onShellStateApplied: renderShellStateRefresh,
  onSyncSuccess: gatewaySyncController.recordSuccess,
  onSyncError: (error) => gatewaySyncController.recordFailure(error, "实时同步失败"),
  refreshFromGateway,
  startPolling: gatewayPollingController.start,
  stopPolling: gatewayPollingController.stop,
});

const messageSendController = createMessageSendController({
  getContext: () => ({
    roomId: activeRoomId,
    gatewayConnected: Boolean(gatewayUrl),
    loginRequired: residentGatewayLoginRequired(),
  }),
  commitLocal: ({ roomId, text, quickAction }) => commitLocalSend(roomId, text, quickAction),
  buildPayload: ({ roomId, text, quickAction }) => gatewayMessagePayload(roomId, text, quickAction),
  prepareGateway: ({ roomId, text, quickAction }) => prepareGatewaySend(roomId, text, quickAction),
  postGateway: ({ payload }) => postGatewayJson("/v1/shell/message", payload),
  clearSendError: ({ roomId }) => { delete roomSendErrors[roomId]; },
  refreshGateway: () => refreshFromGateway({ requireShell: true }),
  clearPending: ({ roomId }) => clearPendingEchoes(roomId),
  handleFailure: ({ roomId, pendingEchoId, posted, error }) => (
    handleGatewaySendFailure(roomId, pendingEchoId, posted, error)
  ),
  onSettled: finishGatewaySendAttempt,
});

function messageSendInFlight() {
  return messageSendController.isSending();
}

applyLocalTimeOfDayState();

function userShellProjection() {
  return currentShellPage() === "user" || document.body?.dataset?.residentLogin === "enabled";
}

function defaultActiveRoomId(rooms = []) { return _defaultActiveRoomId(rooms); }

function shellModeViewState() {
  const vs = _shellModeViewState();
  shellMode = vs.shellMode;
  return vs;
}

function applyShellModeBodyDataset(viewState) {
  _applyShellModeBodyDataset(viewState);
}

function updateShellModeBadge(viewState) {
  _updateShellModeBadge(viewState, shellModeBadgeEl);
}

function updateShellModeDocumentTitle(viewState) {
  _updateShellModeDocumentTitle(viewState);
}

function updateShellModeMasthead(viewState) {
  _updateShellModeMasthead(viewState, {
    eyebrowEl: mastheadEyebrowEl,
    titleEl: mastheadTitleEl,
    heroEl: heroNoteEl,
  });
}

function renderShellModeGuide(config) {
  _renderShellModeGuide(config, modeGuideEl);
}

function toggleShellModeEntryGrid(shellPage) {
  _toggleShellModeEntryGrid(shellPage, entryGridEl);
}

function toggleShellModeStatusChrome(compactShell) {
  _toggleShellModeStatusChrome(compactShell, {
    transportEl: transportStateEl,
    storageEl: storageStateEl,
    gatewayEl: gatewayStateEl,
    providerEl: providerStateEl,
    worldEl: worldStateEl,
  });
}

function toggleAdminShellRoleVisibility(hideAdmin) {
  _toggleAdminShellRoleVisibility(hideAdmin);
}

function applyShellMode() {
  const viewState = shellModeViewState();
  applyShellModeBodyDataset(viewState);
  updateShellEntryCards(viewState.shellMode);
  updateShellModeBadge(viewState);
  updateShellModeDocumentTitle(viewState);
  updateShellModeMasthead(viewState);
  renderShellModeGuide(viewState.config);
  toggleShellModeEntryGrid(viewState.shellPage);
  toggleShellModeStatusChrome(viewState.compactShell);
  toggleAdminShellRoleVisibility(viewState.shellMode === "user");
  updatePanelTitles();
  ensureConversationCallout();
  updateConversationCallout();
}

function updateShellEntryCards(mode) {
  _updateShellEntryCards(mode, shellEntryCards);
}

function workspaceStorageKey()       { return _workspaceStorageKey(currentShellPage(), shellMode); }
function chatPaneStorageKey()         { return _chatPaneStorageKey(currentShellPage(), shellMode); }

function resolveWorkspace() {
  return resolveWorkspaceFromState(
    currentShellPage(),
    shellMode,
    new URL(window.location.href),
    safeLocalStorageGet(workspaceStorageKey()),
  );
}

function defaultChatPaneForViewport() {
  return defaultChatPaneForViewportFromState(
    (query) => window.matchMedia(query),
    activeRoomId,
  );
}

function resolveChatPaneMode() {
  return resolveChatPaneModeFromState(
    currentShellPage(),
    shellMode,
    defaultChatPaneForViewport(),
  );
}

function loadRoomReadMarkers() {
  return loadRoomReadMarkersFromState(currentShellPage(), shellMode);
}

function loadRoomDrafts() {
  return loadRoomDraftsFromState(currentShellPage(), shellMode);
}

function loadRoomQuickStates() {
  return loadRoomQuickStatesFromState(currentShellPage(), shellMode);
}

function loadRoomQuickSnapshots() {
  return loadRoomQuickSnapshotsFromState(currentShellPage(), shellMode);
}

function persistRoomReadMarkers() {
  persistRoomReadMarkersToStorage(currentShellPage(), shellMode, roomReadMarkers);
}

function draftForRoom(roomId) {
  return draftForRoomFromState(roomId, roomDrafts);
}

function roomHasDraft(roomId) {
  return roomHasDraftFromState(roomId, roomDrafts);
}

function updateRoomDraft(roomId, value) {
  roomDrafts = updateRoomDraftInState(
    roomId,
    value,
    roomDrafts,
    currentShellPage(),
    shellMode,
  );
}

const pendingEchoesForRoom = (roomId) => pendingEchoStore.forRoom(roomId);
const enqueuePendingEcho = (roomId, text, quickAction = "") => (
  pendingEchoStore.enqueue(roomId, text, quickAction)
);
const markPendingEchoFailed = (roomId, echoId, failed) => (
  pendingEchoStore.markFailed(roomId, echoId, failed)
);
const removePendingEcho = (roomId, echoId) => pendingEchoStore.remove(roomId, echoId);
const clearPendingEchoes = (roomId) => pendingEchoStore.clearRoom(roomId);
const clearAllPendingEchoes = () => pendingEchoStore.clearAll();

function visiblePendingEchoesForRoom(room) {
  return visiblePendingEchoesForRoomData(room, pendingEchoesForRoom(room?.id));
}

function visiblePendingEchoCount(room) {
  return visiblePendingEchoesForRoom(room).length;
}

// messageStableId moved to shell-message-render.js

function clearMessageEditTarget({ clearInput = false } = {}) {
  editingMessageTarget = null;
  if (composerFormEl) {
    delete composerFormEl.dataset.editingMessageId;
    delete composerFormEl.dataset.editingRoomId;
  }
  if (clearInput && composerInputEl) {
    composerInputEl.value = "";
    autoSizeComposerInput();
  }
}

function beginMessageEdit(room, message) {
  const messageId = messageStableId(message);
  if (!gatewayUrl || !room?.id || !messageId || message?.is_recalled || message?.moderation_status === 'blocked') return;
  editingMessageTarget = {
    roomId: room.id,
    messageId,
    text: typeof message.text === "string" ? message.text : "",
  };
  activeRoomId = room.id;
  if (composerFormEl) {
    composerFormEl.dataset.editingMessageId = messageId;
    composerFormEl.dataset.editingRoomId = room.id;
  }
  if (composerInputEl) {
    composerInputEl.value = editingMessageTarget.text;
    composerInputEl.dispatchEvent(new Event("input", { bubbles: true }));
    requestAnimationFrame(() => {
      composerInputEl?.focus({ preventScroll: true });
      composerInputEl?.select?.();
    });
  }
  updateComposerState();
}

async function recallCommittedMessage(room, message, button = null) {
  const messageId = messageStableId(message);
  if (!gatewayUrl || !room?.id || !messageId || message?.is_recalled) return false;
  if (typeof window.confirm === "function" && !window.confirm("撤回这条消息？")) {
    return false;
  }
  if (button) button.disabled = true;
  try {
    await recallMessage(room.id, messageId);
    if (editingMessageTarget?.messageId === messageId) {
      clearMessageEditTarget({ clearInput: true });
    }
    delete roomSendErrors[room.id];
    renderRooms();
    renderTimeline();
    renderConversationOverview();
    updateComposerState();
    return true;
  } catch (error) {
    roomSendErrors[room.id] = localizedRuntimeError(error, "消息撤回失败");
    refreshGatewayBadge();
    renderRooms();
    renderTimeline();
    renderConversationOverview();
    updateComposerState();
    return false;
  } finally {
    if (button) button.disabled = false;
  }
}

// ── 长按/右键消息动作面板(替代常驻 message-actions) ──
// 时间线当前渲染上下文,供动作面板按 stable id 找回消息。
let lastTimelineContext = { room: null, messages: [] };

function openMessageActionSheetForTarget(target) {
  const article = target.closest?.(".message[data-message-stable-id]");
  if (!article || !article.dataset.messageStableId) return false;
  const { room, messages } = lastTimelineContext;
  if (!room) return false;
  const message = (messages || []).find(
    (item) => messageStableId(item) === article.dataset.messageStableId,
  );
  if (!message) return false;
  const isSelf = String(message.sender || "") === currentIdentity();
  const messageKind = article.dataset.messageKind || "";
  const specs = messageOwnerActionSpecs({ gatewayUrl, isSelf, message, messageKind });
  if (!specs.length) return false;
  return messageActionSheet.open({
    specs: specs.map((spec) => ({
      action: spec.action,
      label: spec.label,
      danger: /danger/.test(spec.className),
    })),
    quoteText: String(message.text || "").slice(0, 40),
    onAction: (action) => {
      if (action === "edit") {
        beginMessageEdit(room, message);
        return;
      }
      if (action === "recall") {
        void recallCommittedMessage(room, message);
      }
    },
  });
}

let pushClientInstance = null;
const messageActionSheet = createMessageActionSheet({ document });

if (timelineEl) {
  document.body.appendChild(messageActionSheet.element);
  document.body.appendChild(
    wireAttachmentLightbox(createAttachmentLightbox({ document }), { document }).element,
  );
  initInstallHint({ document });
  pushClientInstance = initPushClient({ document, gatewayUrl: () => gatewayUrl, getSessionToken: () => getSessionToken() });
  let messageLongPressTimer = null;
  const cancelMessageLongPress = () => clearTimeout(messageLongPressTimer);
  timelineEl.addEventListener("contextmenu", (event) => {
    const article = event.target.closest?.(".message[data-message-stable-id]");
    if (!article) return;
    event.preventDefault();
    openMessageActionSheetForTarget(event.target);
  });
  timelineEl.addEventListener("pointerdown", (event) => {
    if (event.pointerType === "mouse") return;
    cancelMessageLongPress();
    const target = event.target;
    messageLongPressTimer = setTimeout(() => openMessageActionSheetForTarget(target), 500);
  });
  ["pointerup", "pointermove", "pointercancel", "pointerleave"].forEach((type) => {
    timelineEl.addEventListener(type, cancelMessageLongPress);
  });
}

async function retryPendingEcho(roomId, echoId) {
  if (messageSendInFlight()) return false;
  const pending = pendingEchoesForRoom(roomId).find((item) => item.id === echoId);
  if (!pending) return false;
  activeRoomId = roomId;
  removePendingEcho(roomId, echoId);
  delete roomSendErrors[roomId];
  renderRooms();
  renderTimeline();
  renderConversationOverview();
  updateComposerState();
  try {
    await sendMessage(pending.text, { quickAction: pending.quick_action || "" });
    return true;
  } catch (error) {
    roomSendErrors[roomId] = localizedRuntimeError(error, "消息发送失败");
    refreshGatewayBadge();
    renderRooms();
    renderTimeline();
    renderConversationOverview();
    updateComposerState();
    return false;
  }
}

function latestRoomMessageLike(room) { return _latestRoomMessageLike(room); }

const {
  roomQuickStateRecord,
  roomQuickPreviewRecord,
  roomQuickSnapshotHistory,
  roomQuickSnapshot,
  latestRoomQuickSnapshotIndex,
  roomQuickState,
  roomQuickStage,
  roomQuickPreviewState,
  roomQuickPreviewSnapshotIndex,
  roomQuickPreviewFieldView,
  roomQuickPreviewCardFieldView,
  latestStructuredQuickActionPreview,
  latestRoomQuickAction,
  quickActionSendLabel,
  roomQuickPreviewSummary,
  roomQuickPreviewHistoryLabel,
  resolveRoomQuickPreview,
  latestRoomQuickState,
  roomQuickActionSummary,
  roomQuickActionContextCopy,
} = createQuickActionReaders({
  getSnapshots: () => roomQuickSnapshots,
  getPreviews: () => roomQuickStatePreviews,
  getStates: () => roomQuickStates,
  getPendingEchoes: () => pendingEchoStore.snapshot(),
  latestRoomMessageLike,
});

// latestStructuredQuickActionPreview, latestRoomQuickAction, quickActionSendLabel extracted to shell-quick-action-reader.js

function createWorkflowProgress(action, state = "", options = {}) {
  const progressDomSpec = buildWorkflowProgressDomSpec(action, state, options);
  if (!progressDomSpec) return null;

  const progress = document.createElement("div");
  progress.className = progressDomSpec.classNames.join(" ");
  Object.entries(progressDomSpec.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(progress, key, value);
  });

  if (progressDomSpec.titleLine) {
    progress.appendChild(createLine(progressDomSpec.titleLine.className, progressDomSpec.titleLine.text));
  }

  const steps = document.createElement("div");
  steps.className = progressDomSpec.stepsClassName;

  for (const stepSpec of progressDomSpec.steps) {
    const step = document.createElement("div");
    step.className = stepSpec.className;
    Object.entries(stepSpec.dataset || {}).forEach(([key, value]) => {
      setDatasetFlag(step, key, value);
    });
    if (stepSpec.clickable && typeof options.onStageClick === "function") {
      step.setAttribute("role", "button");
      step.tabIndex = 0;
      step.addEventListener("click", () => {
        options.onStageClick(stepSpec.stage, stepSpec.index);
      });
    }

    const marker = document.createElement("span");
    marker.className = stepSpec.markerClassName;
    marker.textContent = stepSpec.markerText;

    const label = document.createElement("span");
    label.className = stepSpec.labelClassName;
    label.textContent = stepSpec.labelText;

    step.append(marker, label);
    steps.appendChild(step);
  }

  progress.appendChild(steps);
  return progress;
}

// roomQuickStateRecord, roomQuickPreviewRecord, roomQuickSnapshotHistory, roomQuickSnapshot,
// latestRoomQuickSnapshotIndex extracted to shell-quick-action-reader.js

function setRoomQuickSnapshot(roomId, action = "", state = "", structured = null) {
  roomQuickSnapshots = setRoomQuickSnapshotInState(
    roomId,
    action,
    state,
    structured,
    roomQuickSnapshots,
    currentShellPage(),
    shellMode,
  );
}

function captureRoomQuickSnapshotFromText(roomId, action = "", state = "", text = "") {
  if (!roomId || !action || !state || !text) return;
  setRoomQuickSnapshot(
    roomId,
    action,
    state,
    parseStructuredQuickActionMessage({
      text,
      quick_action: action,
    }),
  );
}

// roomQuickState, roomQuickStage, roomQuickPreviewState, roomQuickPreviewSnapshotIndex,
// roomQuickPreviewFieldView extracted to shell-quick-action-reader.js

function setRoomQuickState(roomId, action = "", state = "") {
  roomQuickStates = setRoomQuickStateInState(
    roomId,
    action,
    state,
    roomQuickStates,
    currentShellPage(),
    shellMode,
  );
}

function resetRoomQuickState(roomId, action = "") {
  const stages = quickActionStateStages(action);
  if (!roomId || !stages.length) {
    setRoomQuickState(roomId, "", "");
    return "";
  }
  setRoomQuickState(roomId, action, stages[0].label);
  return stages[0].label;
}

// roomQuickStage, roomQuickPreviewState, roomQuickPreviewSnapshotIndex, roomQuickPreviewFieldView
// extracted to shell-quick-action-reader.js

function setRoomQuickPreview(roomId, action = "", state = "", snapshotIndex = null, fieldView = "") {
  if (!roomId || !action || !state) {
    delete roomQuickStatePreviews[roomId];
    return;
  }
  roomQuickStatePreviews[roomId] = {
    action,
    state,
    ...(Number.isInteger(snapshotIndex) && snapshotIndex >= 0 ? { snapshotIndex } : {}),
    ...((fieldView === "stage" || fieldView === "snapshot") ? { fieldView } : {}),
  };
}

function setRoomQuickPreviewFieldView(roomId, action = "", previewState = "", snapshotIndex = null, fieldView = "") {
  if (!roomId || !action || !previewState || (fieldView !== "stage" && fieldView !== "snapshot")) return;
  setRoomQuickPreview(roomId, action, previewState, snapshotIndex, fieldView);
  renderRooms();
  renderConversationOverview();
  renderChatDetailPanel();
  if (currentShellPage() === "user" && roomId === activeRoomId) {
    const room = state.rooms.find((item) => item.id === roomId);
    syncRoomStageCanvas(room);
  }
}

// roomQuickPreviewCardFieldView extracted to shell-quick-action-reader.js

function setRoomQuickPreviewCardFieldView(
  roomId,
  action = "",
  previewState = "",
  snapshotIndex = null,
  fieldView = "",
) {
  if (!roomId || !action || !previewState || (fieldView !== "stage" && fieldView !== "snapshot")) return;
  const record = roomQuickPreviewRecord(roomId) || {};
  roomQuickStatePreviews[roomId] = {
    ...record,
    action,
    state: previewState,
    ...(Number.isInteger(snapshotIndex) && snapshotIndex >= 0 ? { snapshotIndex } : {}),
    cardFieldView: fieldView,
  };
  renderConversationOverview();
  renderChatDetailPanel();
  if (currentShellPage() === "user" && roomId === activeRoomId) {
    const room = state.rooms.find((item) => item.id === roomId);
    syncRoomStageCanvas(room);
  }
}

function previewRoomQuickStage(roomId, action = "", previewState = "", snapshotIndex = null) {
  if (!roomId || !action || !previewState) {
    setRoomQuickPreview(roomId, "", "");
  } else {
    setRoomQuickPreview(roomId, action, previewState, snapshotIndex);
  }
  renderRooms();
  renderConversationOverview();
  renderChatDetailPanel();
  if (currentShellPage() === "user" && roomId === activeRoomId) {
    const room = state.rooms.find((item) => item.id === roomId);
    syncRoomStageCanvas(room);
  }
}

// roomQuickPreviewSummary, roomQuickPreviewHistoryLabel, resolveRoomQuickPreview
// extracted to shell-quick-action-reader.js

function createQuickActionPreviewSummaryLine(preview, options = {}) {
  const summaryLineDomSpec = buildQuickActionPreviewSummaryLineDomSpec(preview, options);
  if (!summaryLineDomSpec) return null;
  const line = document.createElement(summaryLineDomSpec.tagName);
  line.className = summaryLineDomSpec.className;
  summaryLineDomSpec.parts.forEach((partSpec) => {
    const part = document.createElement("span");
    part.className = partSpec.className;
    part.textContent = partSpec.text;
    line.appendChild(part);
  });
  return line;
}

function attachQuickActionPreviewMetaPillAction(pill, title, onActivate) {
  if (!pill || typeof onActivate !== "function") return;
  const clickableDomSpec = quickActionPreviewClickableDomSpec(title);
  clickableDomSpec.classNames.forEach((className) => pill.classList.add(className));
  pill.tabIndex = clickableDomSpec.tabIndex;
  Object.entries(clickableDomSpec.attributes || {}).forEach(([key, value]) => {
    if (key === "title") {
      pill.title = value;
    }
    pill.setAttribute(key, value);
  });
  pill.addEventListener("click", () => {
    onActivate();
  });
  pill.addEventListener("keydown", (event) => {
    if (quickActionPreviewKeyActivates(event.key)) {
      event.preventDefault();
      onActivate();
    }
  });
}

function createQuickActionPreviewPillNode(pillSpec, history, options) {
  const pill = pillSpec.className ? document.createElement("span") : createPill(pillSpec.text, pillSpec.tone);
  if (pillSpec.className) pill.className = pillSpec.className;
  Object.entries(pillSpec.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(pill, key, value);
  });
  pill.textContent = pillSpec.text;
  const pillActionTarget = pillSpec.actionTarget;
  if (pillActionTarget?.kind === "history") {
    attachQuickActionPreviewMetaPillAction(
      pill,
      pillActionTarget.title,
      typeof options.onHistoryClick === "function"
        ? () => {
            options.onHistoryClick(history[pillActionTarget.snapshotIndex], pillActionTarget.snapshotIndex);
          }
        : null,
    );
  }
  if (pillActionTarget?.kind === "field-view") {
    attachQuickActionPreviewMetaPillAction(
      pill,
      pillActionTarget.title,
      typeof options.onFieldViewChange === "function"
        ? () => {
            options.onFieldViewChange(pillActionTarget.fieldView);
          }
        : null,
    );
  }
  return pill;
}

function createQuickActionPreviewPillGroupLabelNode(labelSpec) {
  const label = document.createElement("span");
  label.className = labelSpec.className;
  Object.entries(labelSpec.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(label, key, value);
  });
  label.textContent = labelSpec.text;
  return label;
}

function createQuickActionPreviewPillGroupNode(groupSpec, history, options) {
  const group = document.createElement("div");
  group.className = groupSpec.className;
  Object.entries(groupSpec.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(group, key, value);
  });
  groupSpec.pills.forEach((pillSpec) => {
    group.appendChild(createQuickActionPreviewPillNode(pillSpec, history, options));
  });
  return group;
}

function appendQuickActionPreviewHeader(card, previewRenderDomSpec, history, options) {
  const header = document.createElement("div");
  header.className = previewRenderDomSpec.header.headerClassName;

  const heading = document.createElement("div");
  heading.className = previewRenderDomSpec.header.headingClassName;
  heading.appendChild(createLine(previewRenderDomSpec.header.kickerLine.className, previewRenderDomSpec.header.kickerLine.text));
  heading.appendChild(createLine(previewRenderDomSpec.header.titleLine.className, previewRenderDomSpec.header.titleLine.text));
  header.appendChild(heading);

  const pills = document.createElement("div");
  pills.className = previewRenderDomSpec.pillsWrapperClassName;
  previewRenderDomSpec.pillSections.forEach((sectionSpec) => {
    pills.appendChild(createQuickActionPreviewPillGroupLabelNode(sectionSpec.label));
    pills.appendChild(createQuickActionPreviewPillGroupNode(sectionSpec.group, history, options));
  });

  header.appendChild(pills);
  card.appendChild(header);
}

function createQuickActionPreviewControlButtonNode(buttonDomSpec, history, options) {
  const button = document.createElement("button");
  button.type = buttonDomSpec.type;
  button.className = buttonDomSpec.className;
  Object.entries(buttonDomSpec.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(button, key, value);
  });
  button.textContent = buttonDomSpec.text;
  button.title = buttonDomSpec.title;
  const target = buttonDomSpec.actionTarget;
  if (target?.kind === "history" && typeof options.onHistoryClick === "function") {
    button.addEventListener("click", () => {
      options.onHistoryClick(history[target.snapshotIndex], target.snapshotIndex);
    });
  }
  if (target?.kind === "field-view") {
    button.addEventListener("click", () => {
      if (typeof options.onFieldViewChange === "function") {
        options.onFieldViewChange(target.fieldView);
      }
    });
  }
  return button;
}

function appendQuickActionPreviewControlPanels(card, previewRenderDomSpec, history, options) {
  previewRenderDomSpec.controlPanels.forEach((panelSpec) => {
    const panel = document.createElement("div");
    panel.className = panelSpec.wrapper.className;
    panel.hidden = panelSpec.wrapper.hidden;
    Object.entries(panelSpec.wrapper.attributes || {}).forEach(([key, value]) => {
      panel.setAttribute(key, value);
    });
    if (panelSpec.labelLine) {
      panel.appendChild(createLine(panelSpec.labelLine.className, panelSpec.labelLine.text));
    }
    const buttonNodes = panelSpec.buttons.map((buttonDomSpec) =>
      createQuickActionPreviewControlButtonNode(buttonDomSpec, history, options),
    );
    if (panelSpec.buttonsClassName) {
      const buttons = document.createElement("div");
      buttons.className = panelSpec.buttonsClassName;
      buttonNodes.forEach((button) => buttons.appendChild(button));
      panel.appendChild(buttons);
    } else {
      buttonNodes.forEach((button) => panel.appendChild(button));
    }
    card.appendChild(panel);
  });
}

function createQuickActionPreviewSheetNode(sheetRenderDomSpec) {
  if (!sheetRenderDomSpec) return null;
  const sheet = document.createElement("div");
  sheet.className = sheetRenderDomSpec.wrapperClassName;
  for (const childSpec of sheetRenderDomSpec.children) {
    if (childSpec.kind === "row") {
      const row = document.createElement("div");
      row.className = childSpec.className;

      const label = document.createElement("span");
      label.className = childSpec.label.className;
      label.textContent = childSpec.label.text;

      const value = document.createElement("span");
      value.className = childSpec.value.className;
      value.textContent = childSpec.value.text;

      row.append(label, value);
      sheet.appendChild(row);
    }
    if (childSpec.kind === "notes") {
      const notes = document.createElement("div");
      notes.className = childSpec.className;
      notes.textContent = childSpec.text;
      sheet.appendChild(notes);
    }
  }
  return sheet;
}

function createQuickActionPreviewCard(action, previewState = "", structured = null, options = {}) {
  if (!action || !previewState || !structured) return null;
  const previewCardModel = buildQuickActionPreviewCardModel(action, previewState, structured, {
    ...options,
    followUpCopy: quickActionFollowUpCopy(action, previewState),
  });
  if (!previewCardModel) return null;
  const { history, activeStructured } = previewCardModel;
  if (!activeStructured) return null;
  const previewRenderDomSpec = buildQuickActionPreviewCardRenderDomSpec({
    action,
    previewState,
    previewCardModel,
    className: options.className || "",
    title: options.title || "",
    historyLabel: options.historyLabel || "",
    historyTitle: options.historyTitle || "",
    followUpCopy: quickActionFollowUpCopy(action, previewState),
    maxFields: options.maxFields,
  });
  if (!previewRenderDomSpec) return null;

  const card = document.createElement("section");
  card.className = previewRenderDomSpec.card.classNames.join(" ");
  Object.entries(previewRenderDomSpec.card.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(card, key, value);
  });

  appendQuickActionPreviewHeader(card, previewRenderDomSpec, history, options);

  const copyDomSpec = previewRenderDomSpec.copy;
  if (copyDomSpec) {
    card.appendChild(createLine(copyDomSpec.className, copyDomSpec.text));
  }

  appendQuickActionPreviewControlPanels(card, previewRenderDomSpec, history, options);

  const sheet = createQuickActionPreviewSheetNode(previewRenderDomSpec.sheet);
  if (!sheet) return card;
  card.appendChild(sheet);
  return card;
}

// latestRoomQuickState extracted to shell-quick-action-reader.js

function advanceRoomQuickState(roomId) {
  const room = state.rooms.find((item) => item.id === roomId);
  const action = latestRoomQuickAction(room);
  if (!roomId || !action) return;
  const currentState = roomQuickState(roomId, action);
  const nextState = nextQuickActionState(action, currentState);
  if (!nextState) return;
  setRoomQuickState(roomId, action, nextState);
  setRoomQuickSnapshot(roomId, action, nextState, latestStructuredQuickActionPreview(room, action));
  setRoomQuickPreview(roomId, "", "");
  renderRooms();
  renderTimeline();
  renderConversationOverview();
  renderChatDetailPanel();
  if (currentShellPage() === "user" && roomId === activeRoomId) {
    syncRoomStageCanvas(room);
  }
}

// roomQuickActionSummary, roomQuickActionContextCopy extracted to shell-quick-action-reader.js

function createRoomQuickActionPill(room) {
  const action = latestRoomQuickAction(room);
  const pillDomSpec = buildRoomQuickActionPillDomSpec(action);
  if (!pillDomSpec) return null;
  const pill = createPill(pillDomSpec.text, pillDomSpec.tone);
  Object.entries(pillDomSpec.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(pill, key, value);
  });
  pillDomSpec.classNames.forEach((className) => pill.classList.add(className));
  pill.title = pillDomSpec.title;
  pill.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    if (room.id !== activeRoomId) {
      focusRoom(room.id);
      renderRooms();
      renderTimeline();
    }
    const state = latestRoomQuickState(room);
    seedComposerFromQuickAction(action, quickActionWorkflowTemplate(action, state), { force: true });
  });
  return pill;
}

function createRoomQuickPreviewPill(room) {
  const preview = resolveRoomQuickPreview(room);
  if (!preview?.historyLabel) return null;
  const previewFieldView = roomQuickPreviewFieldView(
    room.id,
    preview.action,
    preview.state,
    preview.snapshotIndex,
  );
  const pillDomSpec = buildRoomQuickPreviewPillDomSpec(preview, previewFieldView);
  if (!pillDomSpec) return null;
  const pill = createPill(pillDomSpec.text, pillDomSpec.tone);
  pillDomSpec.classNames.forEach((className) => pill.classList.add(className));
  Object.entries(pillDomSpec.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(pill, key, value);
  });
  pill.title = pillDomSpec.title;
  pill.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    if (room.id !== activeRoomId) {
      focusRoom(room.id);
      renderRooms();
      renderTimeline();
    }
    previewRoomQuickStage(room.id, preview.action, preview.state, preview.snapshotIndex);
    const previewDraft =
      quickActionStructuredDraft(preview.structured, preview.action) ||
      quickActionWorkflowTemplate(preview.action, preview.state);
    seedComposerFromQuickAction(preview.action, previewDraft, { force: true });
  });
  return pill;
}

function applyInlineClickableDomSpec(node, clickableSpec) {
  if (!node || !clickableSpec) return;
  (clickableSpec.classNames || []).forEach((className) => {
    if (className) node.classList.add(className);
  });
  if (Number.isInteger(clickableSpec.tabIndex)) {
    node.tabIndex = clickableSpec.tabIndex;
  }
  Object.entries(clickableSpec.attributes || {}).forEach(([key, value]) => {
    if (value) node.setAttribute(key, String(value));
  });
}

function createInlineCardContainerNode(containerSpec) {
  const container = document.createElement("div");
  container.className = containerSpec.className;
  if (containerSpec.hidden !== undefined) {
    container.hidden = containerSpec.hidden;
  }
  if (containerSpec.ariaHidden !== undefined) {
    container.setAttribute("aria-hidden", containerSpec.ariaHidden);
  }
  return container;
}

function createInlineCardSimpleChildNode(childSpec) {
  const child = document.createElement(childSpec.type || "div");
  child.className = childSpec.className;
  child.textContent = childSpec.text || "";
  return child;
}

function createInlineCardButtonNode(buttonSpec) {
  const button = document.createElement(buttonSpec.type || "button");
  button.type = buttonSpec.buttonType || "button";
  Object.entries(buttonSpec.dataset || {}).forEach(([key, value]) => {
    button.dataset[key] = String(value);
  });
  button.textContent = buttonSpec.text;
  if (buttonSpec.title) {
    button.title = buttonSpec.title;
  }
  if (buttonSpec.ariaLabel) {
    button.setAttribute("aria-label", buttonSpec.ariaLabel);
  }
  applyInlineClickableDomSpec(button, buttonSpec.clickable);
  return button;
}

function attachInlineMetaPillAction(pill, clickableSpec, onActivate) {
  if (!pill || typeof onActivate !== "function") return;
  applyInlineClickableDomSpec(pill, clickableSpec);
  pill.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    onActivate();
  });
  pill.addEventListener("keydown", (event) => {
    if (quickActionPreviewKeyActivates(event.key)) {
      event.preventDefault();
      event.stopPropagation();
      onActivate();
    }
  });
}

function createInlineMetaChildNode(childSpec, attachInlineMetaModelAction) {
  const node = document.createElement(childSpec.type || "span");
  node.className = childSpec.className;
  Object.entries(childSpec.dataset || {}).forEach(([key, value]) => {
    node.dataset[key] = String(value);
  });
  node.textContent = childSpec.text || "";
  if (childSpec.actionTarget) {
    attachInlineMetaModelAction(node, {
      ...childSpec.actionTarget,
      clickable: childSpec.clickable,
    });
  }
  (childSpec.children || []).forEach((nestedSpec) => {
    node.appendChild(createInlineMetaChildNode(nestedSpec, attachInlineMetaModelAction));
  });
  return node;
}

function appendInlineCardHeader(inlineCard, childModel) {
  (childModel.children || []).forEach((childSpec) => {
    inlineCard.appendChild(createInlineCardSimpleChildNode(childSpec));
  });
}

function appendInlineCardMeta(inlineCard, childModel, attachInlineMetaModelAction) {
  const inlineMetaDomModel = childModel.model;
  if (!inlineMetaDomModel) return;
  const inlineMeta = createInlineCardContainerNode(inlineMetaDomModel);
  inlineMetaDomModel.sections.forEach((section) => {
    (section.children || []).forEach((childSpec) => {
      inlineMeta.appendChild(createInlineMetaChildNode(childSpec, attachInlineMetaModelAction));
    });
  });
  inlineCard.appendChild(inlineMeta);
}

function appendInlineCardActions(inlineCard, childModel, inlineActionHandlers) {
  const inlineActionDomModel = childModel.model;
  if (!inlineActionDomModel) return;
  const inlineActions = createInlineCardContainerNode(inlineActionDomModel);
  (inlineActionDomModel.children || []).forEach((buttonSpec) => {
    const target = buttonSpec.actionTarget;
    const handler = inlineActionHandlers[target?.type];
    if (typeof handler !== "function") return;
    const button = createInlineCardButtonNode(buttonSpec);
    button.addEventListener("click", handler);
    inlineActions.appendChild(button);
  });
  inlineCard.appendChild(inlineActions);
}

function appendInlineCardFieldRows(inlineCard, childModel) {
  const inlineFieldRowsDomModel = childModel.model;
  if (!inlineFieldRowsDomModel) return;
  const fieldList = createInlineCardContainerNode(inlineFieldRowsDomModel);
  for (const rowSpec of inlineFieldRowsDomModel.rows) {
    const row = createInlineCardContainerNode(rowSpec);
    (rowSpec.children || []).forEach((childSpec) => {
      row.appendChild(createInlineCardSimpleChildNode(childSpec));
    });
    fieldList.appendChild(row);
  }
  inlineCard.appendChild(fieldList);
}

function appendInlineCardControls(inlineCard, childModel, onInlineCardControlAction) {
  const inlineControlsDomModel = childModel.model;
  if (!inlineControlsDomModel) return;
  inlineControlsDomModel.groups.forEach((group) => {
    const container = createInlineCardContainerNode(group);
    (group.children || []).forEach((buttonSpec) => {
      const button = createInlineCardButtonNode(buttonSpec);
      button.addEventListener("click", (event) => {
        event.preventDefault();
        event.stopPropagation();
        onInlineCardControlAction(buttonSpec.actionTarget);
      });
      container.appendChild(button);
    });
    inlineCard.appendChild(container);
  });
}

function createInlineHintNode(inlineHintDomModel, applyInlineHintClickable) {
  if (!inlineHintDomModel) return null;
  const hint = document.createElement("div");
  hint.className = inlineHintDomModel.className;
  for (const [key, value] of Object.entries(inlineHintDomModel.dataset)) {
    hint.dataset[key] = value;
  }
  for (const part of inlineHintDomModel.parts) {
    if (part.kind === "separator") {
      hint.append(part.label);
      continue;
    }
    const node = document.createElement("span");
    node.className = part.className;
    node.textContent = part.label;
    if (part.title) {
      node.title = part.title;
    }
    applyInlineHintClickable(node, part);
    hint.appendChild(node);
  }
  return hint;
}

function bindInlineHintAction(node, target, room, preview, inlineHintHandlers) {
  const handler = inlineHintHandlers[target?.type];
  if (typeof handler === "function") {
    node.addEventListener("click", handler);
  }
  if (target?.type === "history") {
    node.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      previewRoomQuickStage(room.id, preview.action, preview.state, target.snapshotIndex);
    });
  }
}

function createInlineHintClickableApplier(room, preview, inlineHintHandlers) {
  return (node, part) => {
    if (!part?.actionTarget) return;
    applyInlineClickableDomSpec(node, part.clickable);
    bindInlineHintAction(node, part.actionTarget, room, preview, inlineHintHandlers);
  };
}

function createRoomInlineProgressNode(progressDomSpec) {
  if (!progressDomSpec) return null;
  const progress = document.createElement(progressDomSpec.type || "div");
  progress.className = progressDomSpec.className;
  Object.entries(progressDomSpec.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(progress, key, value);
  });
  progress.title = progressDomSpec.title;
  progress.tabIndex = progressDomSpec.tabIndex;
  Object.entries(progressDomSpec.attributes || {}).forEach(([key, value]) => {
    progress.setAttribute(key, value);
  });
  (progressDomSpec.children || []).forEach((childSpec) => {
    const child = document.createElement(childSpec.type || "span");
    child.className = childSpec.className;
    child.textContent = childSpec.text;
    progress.appendChild(child);
  });
  return progress;
}

function appendRoomInlineProgressNode(rail, room, action, state, progressDomSpec) {
  const progress = createRoomInlineProgressNode(progressDomSpec);
  if (!progress) return null;
  progress.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    previewRoomQuickStage(
      room.id,
      action,
      state,
      latestRoomQuickSnapshotIndex(room.id, action, state) >= 0
        ? latestRoomQuickSnapshotIndex(room.id, action, state)
        : null,
    );
  });
  rail.appendChild(progress);
  return progress;
}

function activateInlineCardPreviewTarget(room, preview, target) {
  if (target?.type === "history") {
    previewRoomQuickStage(room.id, preview.action, preview.state, target.snapshotIndex);
  }
  if (target?.type === "field-view") {
    setRoomQuickPreviewFieldView(
      room.id,
      preview.action,
      preview.state,
      preview.snapshotIndex,
      target.fieldView,
    );
  }
}

function appendRoomInlinePreviewCard(rail, room, preview, inlinePanelRenderDomModel, inlineActionHandlers) {
  const {
    card: inlineCardDomModel,
    children: inlineCardChildren,
  } = inlinePanelRenderDomModel.card;
  const inlineCard = document.createElement("div");
  inlineCard.className = inlineCardDomModel.className;
  Object.entries(inlineCardDomModel.dataset || {}).forEach(([key, value]) => {
    inlineCard.dataset[key] = String(value);
  });
  const attachInlineMetaModelAction = (pill, target) => {
    if (target?.type === "history" || target?.type === "field-view") {
      attachInlineMetaPillAction(pill, target.clickable, () => {
        activateInlineCardPreviewTarget(room, preview, target);
      });
    }
  };
  const handleInlineCardControlAction = (target) => {
    activateInlineCardPreviewTarget(room, preview, target);
  };
  const inlineCardChildRenderers = {
    header: (childModel) => appendInlineCardHeader(inlineCard, childModel),
    meta: (childModel) => appendInlineCardMeta(inlineCard, childModel, attachInlineMetaModelAction),
    controls: (childModel) => appendInlineCardControls(inlineCard, childModel, handleInlineCardControlAction),
    fieldRows: (childModel) => appendInlineCardFieldRows(inlineCard, childModel),
    actions: (childModel) => appendInlineCardActions(inlineCard, childModel, inlineActionHandlers),
  };
  Object.entries(inlineCardDomModel.datasetFlags || {}).forEach(([key, value]) => {
    setDatasetFlag(inlineCard, key, value);
  });
  (inlineCardChildren || []).forEach((childModel) => {
    const renderChild = inlineCardChildRenderers[childModel.kind];
    if (typeof renderChild === "function") renderChild(childModel);
  });
  rail.appendChild(inlineCard);
  return inlineCard;
}

function createRoomInlineActionNode(action, label, role, onActivate) {
  const actionDomSpec = buildRoomInlineActionDomSpec(action, label, role);
  if (!actionDomSpec) return null;
  const actionNode = document.createElement(actionDomSpec.type || "span");
  actionNode.className = actionDomSpec.className;
  Object.entries(actionDomSpec.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(actionNode, key, value);
  });
  actionNode.textContent = actionDomSpec.text;
  if (Number.isInteger(actionDomSpec.tabIndex)) {
    actionNode.tabIndex = actionDomSpec.tabIndex;
  }
  Object.entries(actionDomSpec.attributes || {}).forEach(([key, value]) => {
    if (value) actionNode.setAttribute(key, String(value));
  });
  actionNode.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    if (typeof onActivate === "function") onActivate();
  });
  return actionNode;
}

function appendRoomInlineActionNodes(rail, room, inlineActionsModel) {
  const {
    action,
    state,
    primarySpec,
    secondarySpec,
    primaryLabel,
    secondaryLabel,
  } = inlineActionsModel;
  const primaryActionNode = createRoomInlineActionNode(action, primaryLabel, "primary", () => {
    const nextAction = primarySpec?.action || action;
    seedComposerFromQuickAction(
      nextAction,
      quickActionWorkflowTemplate(nextAction, primarySpec?.next_state || state),
      { force: true },
    );
  });
  if (primaryActionNode) rail.appendChild(primaryActionNode);

  const secondaryActionNode = createRoomInlineActionNode(action, secondaryLabel, "secondary", () => {
    if (secondarySpec?.next_state) {
      const nextAction = secondarySpec.action || action;
      setRoomQuickAction(room.id, nextAction);
      setRoomQuickState(room.id, nextAction, secondarySpec.next_state);
      renderRooms();
      renderTimeline();
      renderConversationOverview();
      renderChatDetailPanel();
      return;
    }
    advanceRoomQuickState(room.id);
  });
  if (secondaryActionNode) rail.appendChild(secondaryActionNode);
}

function createRoomInlineRailNode(railDomSpec) {
  const rail = document.createElement("div");
  rail.className = railDomSpec.className;
  Object.entries(railDomSpec.dataset || {}).forEach(([key, value]) => {
    setDatasetFlag(rail, key, value);
  });
  return rail;
}

function roomInlinePreviewContext(room, action) {
  const preview = resolveRoomQuickPreview(room, action);
  const selectedFieldView = preview
    ? roomQuickPreviewFieldView(room.id, preview.action, preview.state, preview.snapshotIndex)
    : "";
  const previewView = preview
    ? resolveQuickActionPreviewView(preview, selectedFieldView)
    : null;
  return {
    preview,
    previewView,
    selectedFieldView,
    inlinePanelModel: buildQuickActionInlinePreviewPanelModel({
      preview,
      resolvedPreviewView: previewView,
      selectedFieldView,
      maxFields: 2,
    }),
  };
}

function createRoomInlinePreviewHandlers(room, preview) {
  const activatePreviewSnapshot = (event) => {
    event?.preventDefault?.();
    event?.stopPropagation?.();
    previewRoomQuickStage(room.id, preview.action, preview.state, preview.snapshotIndex);
    const previewDraft =
      quickActionStructuredDraft(preview.structured, preview.action) ||
      quickActionWorkflowTemplate(preview.action, preview.state);
    seedComposerFromQuickAction(preview.action, previewDraft, { force: true });
  };
  const activatePreviewWorkflow = (event) => {
    event?.preventDefault?.();
    event?.stopPropagation?.();
    previewRoomQuickStage(room.id, preview.action, preview.state, preview.snapshotIndex);
    seedComposerFromQuickAction(
      preview.action,
      quickActionWorkflowTemplate(preview.action, preview.state),
      { force: true },
    );
  };
  return {
    snapshot: activatePreviewSnapshot,
    workflow: activatePreviewWorkflow,
  };
}

function appendRoomInlinePreviewPanel(rail, room, action) {
  const previewContext = roomInlinePreviewContext(room, action);
  const { preview, inlinePanelModel } = previewContext;
  if (!inlinePanelModel) return true;
  const inlineActionHandlers = createRoomInlinePreviewHandlers(room, preview);
  const inlinePanelRenderDomModel =
    buildQuickActionInlinePreviewPanelRenderDomModel(inlinePanelModel, quickActionIntensity(action));
  if (!inlinePanelRenderDomModel) return false;

  const inlineHintDomModel = inlinePanelRenderDomModel.hint;
  const applyInlineHintClickable = createInlineHintClickableApplier(room, preview, inlineActionHandlers);
  const hint = createInlineHintNode(inlineHintDomModel, applyInlineHintClickable);
  if (hint) rail.appendChild(hint);
  appendRoomInlinePreviewCard(rail, room, preview, inlinePanelRenderDomModel, inlineActionHandlers);
  return true;
}

function createRoomInlineActions(room) {
  const inlineActionsModel = buildRoomInlineActionsModel({
    roomId: room?.id,
    activeRoomId,
    action: latestRoomQuickAction(room),
    state: latestRoomQuickState(room),
    primarySpec: inlineActionProfile(room, "primary"),
    secondarySpec: inlineActionProfile(room, "secondary"),
  });
  if (!inlineActionsModel) return null;
  const {
    action,
    state,
    railDomSpec,
    progressDomSpec,
  } = inlineActionsModel;
  const rail = createRoomInlineRailNode(railDomSpec);
  const progress = appendRoomInlineProgressNode(rail, room, action, state, progressDomSpec);
  if (!progress) return null;
  if (!appendRoomInlinePreviewPanel(rail, room, action)) return rail;
  appendRoomInlineActionNodes(rail, room, inlineActionsModel);
  return rail;
}

function roomPreviewContext(room) {
  const preview = resolveRoomQuickPreview(room);
  const selectedFieldView = preview
    ? roomQuickPreviewFieldView(room.id, preview.action, preview.state, preview.snapshotIndex)
    : "";
  const previewView = preview
    ? resolveQuickActionPreviewView(preview, selectedFieldView)
    : null;
  return {
    preview,
    previewView,
    field: previewView?.primaryField,
  };
}

function createRoomPreviewFallbackNode(room) {
  return createLine("room-preview", roomPreview(room));
}

function activateRoomPreviewSnapshot(room, preview, event) {
  event.preventDefault();
  event.stopPropagation();
  if (room.id !== activeRoomId) {
    focusRoom(room.id);
    renderRooms();
    renderTimeline();
  }
  previewRoomQuickStage(room.id, preview.action, preview.state, preview.snapshotIndex);
  const previewDraft =
    quickActionStructuredDraft(preview.structured, preview.action) ||
    quickActionWorkflowTemplate(preview.action, preview.state);
  seedComposerFromQuickAction(preview.action, previewDraft, { force: true });
}

function activateRoomPreviewHistorySnapshot(room, preview, index, event) {
  event.preventDefault();
  event.stopPropagation();
  if (room.id !== activeRoomId) {
    focusRoom(room.id);
    renderTimeline();
  }
  previewRoomQuickStage(room.id, preview.action, preview.state, index);
}

function createRoomPreviewShellNode(preview, onActivate) {
  const shell = document.createElement("div");
  shell.className = "room-preview-shell is-interactive";
  shell.dataset.previewState = preview.state;
  shell.dataset.previewRound = preview.historyLabel || "";
  shell.title = "点击回到当前预览快照并继续填写";
  shell.addEventListener("click", onActivate);
  return shell;
}

function createRoomPreviewHistoryChipNode(room, preview, snapshot, index) {
  const chip = document.createElement("span");
  chip.className = "room-preview-history-chip";
  chip.dataset.selected = index === preview.snapshotIndex ? "true" : "false";
  chip.dataset.snapshotIndex = String(index);
  chip.dataset.snapshotRole = index === preview.history.length - 1 ? "latest" : "history";
  chip.textContent = quickActionPreviewHistoryLabel(snapshot, index, preview.history.length);
  chip.title = quickActionPreviewHistoryDescription(snapshot, index, preview.history.length);
  chip.addEventListener("click", (event) => {
    activateRoomPreviewHistorySnapshot(room, preview, index, event);
  });
  return chip;
}

function appendRoomPreviewHistoryNodes(shell, room, preview) {
  if (!Array.isArray(preview.history) || preview.history.length <= 1) return;
  const history = document.createElement("div");
  history.className = "room-preview-history";
  preview.history.forEach((snapshot, index) => {
    history.appendChild(createRoomPreviewHistoryChipNode(room, preview, snapshot, index));
  });
  shell.appendChild(history);
}

function createRoomPreviewStageNode(field, previewView, onActivate) {
  const stage = document.createElement("div");
  stage.className = "room-preview-stage";
  stage.textContent = field.label || previewView.state || "预览";
  stage.addEventListener("click", onActivate);
  return stage;
}

function createRoomPreviewSummaryNode(room, field, onActivate) {
  const summary = document.createElement("div");
  summary.className = "room-preview";
  summary.textContent = field.value || field.label || roomPreview(room);
  summary.addEventListener("click", onActivate);
  return summary;
}

function appendRoomPreviewFieldNodes(shell, room, previewView, field, onActivate) {
  shell.appendChild(createRoomPreviewStageNode(field, previewView, onActivate));
  shell.appendChild(createRoomPreviewSummaryNode(room, field, onActivate));
}

function createRoomPreviewNode(room) {
  const context = roomPreviewContext(room);
  if (!context.preview || !context.previewView || !context.field) return createRoomPreviewFallbackNode(room);
  const activatePreview = (event) => {
    activateRoomPreviewSnapshot(room, context.preview, event);
  };
  const shell = createRoomPreviewShellNode(context.preview, activatePreview);
  appendRoomPreviewHistoryNodes(shell, room, context.preview);
  appendRoomPreviewFieldNodes(shell, room, context.previewView, context.field, activatePreview);
  return shell;
}

function renderTimelineSkeletonRows(count = 4) {
  if (!timelineEl) return;
  for (let index = 0; index < count; index += 1) {
    const isSelf = index % 2 === 1;
    const row = document.createElement("div");
    row.className = `message-row timeline-skeleton-row${isSelf ? " self" : ""}`;
    row.dataset.messageKind = "skeleton";
    row.dataset.messageSide = isSelf ? "self" : "peer";

    const avatar = document.createElement("div");
    avatar.className = `message-avatar timeline-skeleton-avatar${isSelf ? " message-avatar-self" : ""}`;
    applyAvatarStyle(avatar, isSelf ? currentIdentity() : "timeline-skeleton");

    const stack = document.createElement("div");
    stack.className = "message-stack";

    const bubble = document.createElement("div");
    bubble.className = "timeline-skeleton-bubble";
    bubble.setAttribute("aria-hidden", "true");
    bubble.setAttribute("style", `--skeleton-width:${index === 0 ? 72 : index === 1 ? 54 : index === 2 ? 64 : 44}%`);

    const linePrimary = document.createElement("span");
    linePrimary.className = "timeline-skeleton-line timeline-skeleton-line-primary";
    const lineSecondary = document.createElement("span");
    lineSecondary.className = "timeline-skeleton-line timeline-skeleton-line-secondary";
    bubble.appendChild(linePrimary);
    bubble.appendChild(lineSecondary);

    stack.appendChild(bubble);
    row.appendChild(avatar);
    row.appendChild(stack);
    timelineEl.appendChild(row);
  }
}

function roomOverviewSummary(room) {
  const action = latestRoomQuickAction(room);
  const actionSummary = quickActionOverviewSummary(action);
  if (actionSummary) return actionSummary;
  if (typeof room?.overview_summary === "string" && room.overview_summary.trim()) {
    return room.overview_summary.trim();
  }
  if (typeof room?.thread_headline === "string" && room.thread_headline.trim()) {
    return room.thread_headline.trim();
  }
  return room.title || room.participant_label || "会话摘要";
}

function appendRoomQuickActionOverviewButton(actions, room, options = {}) {
  const action = latestRoomQuickAction(room);
  const state = latestRoomQuickState(room);
  const primarySpec = inlineActionProfile(room, "primary");
  const nextAction = primarySpec?.action || action;
  const nextState = primarySpec?.next_state || state;
  const label = primarySpec?.label || quickActionOverviewCtaLabel(action, state);
  if (!actions || !label) return;
  const button = document.createElement("button");
  button.type = "button";
  if (options.className) {
    button.className = options.className;
  }
  if (options.dataset) {
    Object.assign(button.dataset, options.dataset);
  }
  button.textContent = label;
  button.addEventListener("click", () => {
    previewRoomQuickStage(room?.id, nextAction, nextState);
    seedComposerFromQuickAction(nextAction, quickActionWorkflowTemplate(nextAction, nextState), { force: true });
  });
  actions.appendChild(button);
}

function unreadCount(room) {
  const seen = Number(roomReadMarkers?.[room.id] || 0);
  return Math.max((room?.messages?.length || 0) - seen, 0);
}

function markRoomRead(roomId) {
  const room = state.rooms.find((item) => item.id === roomId);
  if (!room) return;
  roomReadMarkers[roomId] = room.messages?.length || 0;
  persistRoomReadMarkers();
}

function syncChatPaneMode(mode, { persist = true } = {}) {
  const allowed = mode === "list" || mode === "thread" || mode === "split";
  chatPaneMode = allowed ? mode : defaultChatPaneForViewport();
  if (persist) {
    safeLocalStorageSet(chatPaneStorageKey(), chatPaneMode);
  }
  if (currentWorkspace === "chat") {
    document.body.dataset.chatPane = chatPaneMode;
  }
}

function createWorkspaceButton(workspace) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "workspace-tab";
  button.dataset.workspace = workspace;
  button.textContent = translateWorkspace(workspace);
  button.addEventListener("click", () => {
    setWorkspace(workspace);
  });
  return button;
}

function createRoomFilterButton(filter, label) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "segment";
  button.dataset.roomFilter = filter;
  button.textContent = label;
  button.addEventListener("click", () => {
    roomFilter = filter;
    updateRoomToolbarState();
    renderRooms();
    renderTimeline();
  });
  return button;
}

function createSearchModeButton(mode, label) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "segment";
  button.dataset.searchMode = mode;
  button.textContent = label;
  button.addEventListener("click", () => {
    searchMode = mode;
    updateSearchModeTabs();
    renderRooms();
    if (currentShellPage() === "user") renderResidentList();
  });
  return button;
}

function updateSearchModeTabs() {
  if (!searchModeSegments) return;
  for (const seg of searchModeSegments) {
    seg.classList.toggle("active", seg.dataset.searchMode === searchMode);
  }
  if (roomSearchInputEl) {
    if (searchMode === "rooms") roomSearchInputEl.placeholder = "搜索房间...";
    else if (searchMode === "residents") roomSearchInputEl.placeholder = "搜索居民...";
    else roomSearchInputEl.placeholder = "搜索居民或房间...";
  }
}

function syncWorkspaceNavigationChrome(userProjection, hubProjection) {
  if ((userProjection || hubProjection) && workspaceNavEl) {
    workspaceNavEl.remove();
    workspaceNavEl = null;
    workspaceTabs = [];
  }

  if (!userProjection && !hubProjection && !workspaceNavEl && appShellEl && topbarEl) {
    workspaceNavEl = createWorkspaceNavElement();
  }

  renderWorkspaceNavigationTabs();
}

function createWorkspaceNavElement() {
  const nav = document.createElement("nav");
  nav.className = currentShellPage() === "unified"
    ? "workspace-switcher panel"
    : "workspace-switcher";
  nav.setAttribute("aria-label", "工作区切换");
  topbarEl.insertAdjacentElement("afterend", nav);
  return nav;
}

function renderWorkspaceNavigationTabs() {
  if (!workspaceNavEl) return;
  workspaceNavEl.replaceChildren();
  workspaceTabs = availableWorkspacesForShellMode(shellMode).map((workspace) => {
    const button = createWorkspaceButton(workspace);
    workspaceNavEl.appendChild(button);
    return button;
  });
}

function createRoomSearchInput(config) {
  const input = document.createElement("input");
  if (config.id) input.id = config.id;
  input.type = config.type || "search";
  input.className = config.className;
  input.placeholder = config.placeholder;
  input.autocomplete = "off";
  return input;
}

function ensureUserRoomSearchControls(userProjection) {
  if (!userProjection || roomSearchInputEl || !roomListEl) return;
  roomSearchInputEl = createRoomSearchInput({
    id: "room-search-input",
    className: "creative-rail-search",
    placeholder: "搜索居民、房间或最近消息",
  });
  roomListEl.insertAdjacentElement("beforebegin", roomSearchInputEl);
  ensureSearchModeControls();
}

function ensureSearchModeControls() {
  if (searchModeControlsEl || !roomSearchInputEl) return;
  searchModeControlsEl = document.createElement("div");
  searchModeControlsEl.className = "segmented-control creative-search-mode";
  searchModeSegments = [
    createSearchModeButton("all", "全部"),
    createSearchModeButton("rooms", "房间"),
    createSearchModeButton("residents", "居民"),
  ];
  for (const seg of searchModeSegments) {
    searchModeControlsEl.appendChild(seg);
  }
  roomSearchInputEl.insertAdjacentElement("beforebegin", searchModeControlsEl);
  updateSearchModeTabs();
}

function ensureRoomToolbarChrome(userProjection) {
  if (userProjection || roomSearchInputEl || !roomsPanelEl || !roomListEl) return;
  roomListEl.insertAdjacentElement("beforebegin", createRoomToolbar());
}

function createRoomToolbar() {
  const toolbar = document.createElement("div");
  toolbar.className = "panel-toolbar room-toolbar";

  roomSearchInputEl = createRoomSearchInput({
    className: "search-input",
    placeholder: "搜索频道、私信或最近发言",
  });
  bindRoomSearchInput();

  roomToolbarNoteEl = document.createElement("div");
  roomToolbarNoteEl.className = "toolbar-note";

  toolbar.appendChild(roomSearchInputEl);
  toolbar.appendChild(createRoomFilterSegments());
  toolbar.appendChild(roomToolbarNoteEl);
  return toolbar;
}

function createRoomFilterSegments() {
  const segments = document.createElement("div");
  segments.className = "segmented-control";
  roomFilterButtons = [
    createRoomFilterButton("all", "全部"),
    createRoomFilterButton("direct", "私信"),
    createRoomFilterButton("public", "频道"),
  ];
  for (const button of roomFilterButtons) {
    segments.appendChild(button);
  }
  return segments;
}

function ensureConversationOverviewChrome() {
  if (conversationOverviewEl || !conversationPanelEl || !metaEl) return;
  conversationOverviewEl = document.createElement("section");
  conversationOverviewEl.className = "conversation-overview";
  metaEl.insertAdjacentElement("beforebegin", conversationOverviewEl);
}

function ensureComposerStatusChrome() {
  if (composerStatusEl || !composerFormEl) return;
  composerStatusEl = document.createElement("div");
  composerStatusEl.className = "composer-status composer-status-muted";
  const composerRow = composerFormEl.querySelector(".composer-row");
  if (composerRow) {
    composerRow.insertAdjacentElement("beforebegin", composerStatusEl);
  } else {
    composerFormEl.appendChild(composerStatusEl);
  }
}

function ensureComposerHeroChrome(userProjection) {
  if (userProjection || composerHeroEl || !composerFormEl) return;
  composerHeroEl = document.createElement("div");
  composerHeroEl.className = "composer-hero";
  const composerRow = composerFormEl.querySelector(".composer-row");
  const anchor = composerStatusEl || composerRow;
  if (anchor) {
    anchor.insertAdjacentElement("afterend", composerHeroEl);
  } else {
    composerFormEl.appendChild(composerHeroEl);
  }
}

function ensureComposerContextChrome(userProjection) {
  if (userProjection || composerContextEl || !composerFormEl) return;
  composerContextEl = document.createElement("div");
  composerContextEl.className = "composer-context";
  const anchor = composerHeroEl || composerStatusEl || composerFormEl.querySelector(".composer-row");
  if (anchor) {
    anchor.insertAdjacentElement("afterend", composerContextEl);
  } else {
    composerFormEl.appendChild(composerContextEl);
  }
}

function ensureRoomDigestChrome(userProjection) {
  if (userProjection || roomDigestEl || !roomListEl) return;
  roomDigestEl = document.createElement("div");
  roomDigestEl.className = "room-digest";
  roomListEl.insertAdjacentElement("beforebegin", roomDigestEl);
}

function ensureThreadStatusRailChrome(userProjection) {
  if (userProjection || threadStatusRailEl || !timelineEl) return;
  threadStatusRailEl = document.createElement("div");
  threadStatusRailEl.className = "thread-status-rail";
  timelineEl.insertAdjacentElement("beforebegin", threadStatusRailEl);
}

function ensureComposerMetaChrome(userProjection) {
  if (userProjection || composerMetaEl || !composerFormEl) return;
  composerMetaEl = document.createElement("div");
  composerMetaEl.className = "composer-meta";
  composerFormEl.appendChild(composerMetaEl);
}

function ensureRoomViewToggleChrome(userProjection) {
  if (userProjection || roomViewToggleButtonEl || !conversationPanelEl) return;
  roomViewToggleButtonEl = document.createElement("button");
  roomViewToggleButtonEl.type = "button";
  roomViewToggleButtonEl.className = "secondary conversation-toggle";
  roomViewToggleButtonEl.addEventListener("click", handleRoomViewToggleClick);
  conversationPanelEl.insertAdjacentElement("afterbegin", roomViewToggleButtonEl);
}

function handleRoomViewToggleClick() {
  syncChatPaneMode("list");
  applyWorkspace();
}

function ensureWorkspaceAuxiliaryChrome(userProjection) {
  ensureConversationOverviewChrome();
  ensureComposerStatusChrome();
  ensureComposerHeroChrome(userProjection);
  ensureComposerContextChrome(userProjection);
  ensureRoomDigestChrome(userProjection);
  ensureThreadStatusRailChrome(userProjection);
  ensureComposerMetaChrome(userProjection);
  ensureComposerTip();
  ensureComposerKeyBindings();
  ensureRoomViewToggleChrome(userProjection);
}

function ensureNonUserCaretakerChrome(userProjection) {
  if (userProjection) return;
  ensureCaretakerPanel();
  ensureCaretakerBadge();
}

function ensureWorkspaceChrome() {
  const userProjection = currentShellPage() === "user";
  const hubProjection = currentShellPage() === "hub";

  syncWorkspaceNavigationChrome(userProjection, hubProjection);
  ensureUserRoomSearchControls(userProjection);
  bindRoomSearchInput();
  ensureRoomToolbarChrome(userProjection);
  ensureWorkspaceAuxiliaryChrome(userProjection);
  updateRoomToolbarState();
  ensureNonUserCaretakerChrome(userProjection);
}

function bindRoomSearchInput() {
  if (!roomSearchInputEl || roomSearchInputEl.dataset.roomSearchBound === "true") return;
  roomSearchInputEl.addEventListener("input", (event) => {
    roomSearch = event.target.value.trim().toLowerCase();
    updateRoomToolbarState();
    renderRooms();
    if (currentShellPage() === "user") renderResidentList();
    renderTimeline();
  });
  roomSearchInputEl.dataset.roomSearchBound = "true";
}
function ensureRoomQuickActions() {
  if (currentShellPage() === "user" || !roomsPanelEl) return;
  let actions = roomsPanelEl.querySelector(".room-actions");
  let primary = null;
  let secondary = null;
  if (!actions) {
    actions = document.createElement("div");
    actions.className = "room-actions";
    primary = document.createElement("button");
    primary.type = "button";
    primary.className = "primary";
    primary.dataset.roomAction = "primary";
    primary.addEventListener("click", () => {
      directPeerInputEl?.focus();
      directPeerInputEl?.scrollIntoView({ behavior: "smooth", block: "center" });
    });
    secondary = document.createElement("button");
    secondary.type = "button";
    secondary.className = "secondary";
    secondary.dataset.roomAction = "secondary";
    secondary.addEventListener("click", () => {
      setWorkspace(currentShellPage() === "admin" ? "governance" : "world");
    });
    actions.append(primary, secondary);
    const title = roomsPanelEl.querySelector(".panel-title");
    roomsPanelEl.insertAdjacentElement("beforeend", actions);
  }
  primary = primary || actions.querySelector('[data-room-action="primary"]');
  secondary = secondary || actions.querySelector('[data-room-action="secondary"]');
  const shellPage = currentShellPage();
  if (primary) {
    primary.textContent = shellPage === "admin" ? "打开追问私信" : "发起新私信";
  }
  if (secondary) {
    secondary.textContent = shellPage === "admin" ? "打开更多" : "去看看群聊";
  }
  actions.classList.toggle("is-hidden", currentWorkspace !== "chat");
}

function ensureCaretakerPanel() {
  if (currentShellPage() === "user" || !sidebarStackEl) return;
  if (!caretakerPanelEl) {
    caretakerPanelEl = document.createElement("section");
    caretakerPanelEl.className = "panel caretaker-panel";
    sidebarStackEl.insertBefore(caretakerPanelEl, governancePanelEl || null);
  }
  renderCaretakerPanel();
}

function createCaretakerPanelTitleNode(model) { return _createCaretakerPanelTitleNode(model); }

function createCaretakerPanelHeaderNode(model) { return _createCaretakerPanelHeaderNode(model); }

function createCaretakerPanelSummaryNode(model) { return _createCaretakerPanelSummaryNode(model); }

function createCaretakerMessageNode(item) { return _createCaretakerMessageNode(item); }

function createCaretakerMessagesNode(model) { return _createCaretakerMessagesNode(model); }

function createCaretakerRulesNode(model) { return _createCaretakerRulesNode(model); }

function renderCaretakerPanel() {
  if (!caretakerPanelEl) return;
  clearChildren(caretakerPanelEl);

  const model = caretakerPanelModel();
  caretakerPanelEl.appendChild(createCaretakerPanelTitleNode(model));
  caretakerPanelEl.appendChild(_renderCaretakerPanelBody(model));
}

function ensureCaretakerBadge() {
  if (currentShellPage() === "user" || !composerStatusEl) return;
  if (!caretakerStatusEl) {
    caretakerStatusEl = document.createElement("div");
    caretakerStatusEl.className = "caretaker-status-line";
    composerStatusEl.insertAdjacentElement("afterend", caretakerStatusEl);
  }
  updateCaretakerStatus();
}

function updateCaretakerStatus() {
  if (!caretakerStatusEl) return;
  const room = state.rooms.find((item) => item.id === activeRoomId);
  const roomLabel = room
    ? roomThreadHeadline(room)
    : "等待选中会话";
  clearChildren(caretakerStatusEl);
  const items = caretakerStatusItems({ roomLabel });
  for (const item of items) {
    const node = document.createElement(item.element);
    if (item.className) node.className = item.className;
    node.textContent = item.text;
    caretakerStatusEl.appendChild(node);
  }
}

function ensureChatPriorityBadge() {
  if (currentShellPage() === "user" || !conversationPanelEl) return;
  let badge = conversationPanelEl.querySelector(".chat-priority-badge");
  if (!badge) {
    badge = document.createElement("div");
    badge.className = "chat-priority-badge";
    const title = conversationPanelEl.querySelector(".panel-title");
    if (title) {
      title.insertAdjacentElement("afterend", badge);
    } else {
      conversationPanelEl.prepend(badge);
    }
  }
  updateChatPriorityBadgeText();
}

function updateChatPriorityBadgeText(active = chatFocusController.isActive()) {
  const badge = conversationPanelEl?.querySelector(".chat-priority-badge");
  if (!badge) return;
  badge.textContent = active
    ? "聊天专注 · 按钮可还原"
    : chatPriorityBadgeDefaultText(shellMode);
}

function ensureChatQuickLinks() {
  if (currentShellPage() === "user" || !conversationPanelEl) return;
  const existing = conversationPanelEl.querySelector(".chat-quick-links");
  if (existing && existing.dataset.mode === shellMode) return;
  if (existing) {
    existing.remove();
  }
  const quickLinks = document.createElement("div");
  quickLinks.className = "chat-quick-links";
  quickLinks.dataset.mode = shellMode;
  const targets = chatQuickLinksTargets(shellMode);
  for (const [label, workspace] of targets) {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.workspace = workspace;
    button.textContent = label;
    button.addEventListener("click", () => {
      setWorkspace(workspace);
    });
    quickLinks.appendChild(button);
  }
  const reference =
    conversationPanelEl.querySelector(".chat-priority-badge") ||
    conversationPanelEl.querySelector(".panel-title");
  if (reference) {
    reference.insertAdjacentElement("afterend", quickLinks);
  } else {
    conversationPanelEl.prepend(quickLinks);
  }
}

function updateChatQuickLinksVisibility() {
  if (!conversationPanelEl) return;
  const quickLinks = conversationPanelEl.querySelector(".chat-quick-links");
  if (!quickLinks) return;
  quickLinks.style.display = currentWorkspace === "chat" ? "flex" : "none";
}

function ensureModeBanner() {
  const shellPage = currentShellPage();
  modeBannerEl = ensureModeBannerDom(shellPage, roomsPanelEl);
  if (modeBannerEl) updateModeBanner();
}

function updateModeBanner() {
  if (!modeBannerEl) return;
  modeBannerEl.textContent = modeBannerText(shellMode);
  modeBannerEl.dataset.variant = shellMode;
}

function ensureConversationCallout() {
  const shellPage = currentShellPage();
  conversationCalloutEl = ensureConversationCalloutDom(shellPage, conversationPanelEl, timelineEl);
  if (conversationCalloutEl) updateConversationCallout();
}

function updateConversationCalloutStageTitle(room) {
  if (!roomStageTitleEl) return;
  roomStageTitleEl.textContent = room
    ? roomThreadHeadline(room)
    : "房间内聊天主界面";
}

function conversationCalloutModel(room, caretaker) {
  return conversationCalloutModelForState(room, caretaker, shellMode, {
    roomThreadHeadline,
    roomAudienceLabel,
    roomRouteLabel,
    roomChatStatusSummary,
    roomQueueSummary,
    roomContextSummary,
    caretakerPendingCount,
  });
}

function createConversationCalloutParagraphNode(paragraph) {
  return _createConversationCalloutParagraphNode(paragraph);
}

function renderConversationCalloutContent(model) {
  _renderConversationCalloutContent(model, conversationCalloutEl);
}

function updateConversationCallout() {
  if (currentShellPage() === "user" || !conversationCalloutEl) return;
  conversationCalloutEl.style.display = currentWorkspace === "chat" ? "" : "none";
  const room = state.rooms.find((item) => item.id === activeRoomId);
  const caretaker = caretakerProfile(room);
  updateConversationCalloutStageTitle(room);
  const model = conversationCalloutModel(room, caretaker);
  renderConversationCalloutContent(model);
}

function syncRoomStageCanvas(room) {
  const shellPage = currentShellPage();
  if (shellPage === "hub") {
    syncHubStageCanvas(room);
    return;
  }
  if (shellPage !== "user") return;
  ensureUserSceneChrome();
  if (!roomStageCanvasEl) return;

  if (!room) {
    syncPersonalRoomAccessPolicyControl();
    renderDefaultUserRoomStageCanvas();
    return;
  }

  renderUserRoomStageCanvas(room);
  syncPersonalRoomAccessPolicyControl();
}

function renderDefaultUserRoomStageCanvas() {
  const hudTitleEl = document.querySelector("#room-stage-title");
  const defaultVisual = defaultUserRoomStageVisual();
  if (hudTitleEl) {
    hudTitleEl.textContent = "房间聊天";
  }
  renderStageCanvas(roomStageCanvasEl, defaultVisual);
  roomStageCanvasEl.dataset.variant = "home";
}

function defaultUserRoomStageVisual() {
  return {
    kind: "stage",
    variant: "home",
    title: "房间聊天",
    summary: "回到住处后继续一对一交谈。",
    theme: {
      kicker: "住宅 / 私聊",
      titleFont: '700 22px "Noto Serif SC", "Microsoft YaHei", serif',
      bodyFont: '500 14px "Noto Sans SC", "Microsoft YaHei", sans-serif',
      lineHeightTitle: 30,
      lineHeightBody: 22,
      background: "#4a3525",
      accent: "#d38d4c",
      panel: "rgba(90, 62, 42, 0.8)",
      border: "rgba(211, 141, 76, 0.38)",
      title: "#f7ead7",
      body: "rgba(246, 231, 210, 0.88)",
    },
    visual: {
      motif: "courtyard",
      badge: "住宅 / 私聊",
      signalCount: 2,
    },
  };
}

function buildUserRoomVisual(room) {
  return buildRoomVisualModel(
    room,
    roomStageSummary(room),
    {
      title: roomStagePortraitTitle(room),
      summary: roomStagePortraitSummary(room),
    },
  );
}

function renderUserRoomStageCanvas(room) {
  const visual = buildUserRoomVisual(room);
  if (roomStageTitleEl) {
    roomStageTitleEl.textContent = visual.stage.title;
  }
  const rendered = renderStageCanvas(roomStageCanvasEl, visual.stage);
  roomStageCanvasEl.dataset.variant = visual.stage.variant;
  syncUserRoomProjection(room, visual);
  updateRoomStageNote(rendered, visual.stage.summary);
  renderRoomStagePortrait(room);
}

function updateRoomStageNote(rendered, summary) {
  if (!roomStageNoteEl) return;
  roomStageNoteEl.textContent = summary;
  roomStageNoteEl.style.display = rendered ? "none" : "";
}

function syncHubStageCanvas(room) {
  const canvasEl = document.querySelector("#room-stage-canvas");
  if (!canvasEl) return;
  const hudTitleEl = document.querySelector("#room-stage-title");
  if (room) {
    const visual = buildRoomVisualModel(room, roomStageSummary(room), null);
    if (hudTitleEl) {
      hudTitleEl.textContent = visual.stage.title;
    }
    renderStageCanvas(canvasEl, visual.stage);
    canvasEl.dataset.variant = visual.stage.variant;
  } else {
    const defaultVisual = {
      kind: "stage",
      variant: "city",
      title: "城邦公共频道",
      summary: "公告、闲聊和跨城讨论会先落在这里。",
      theme: {
        kicker: "城市 / 公共频道",
        titleFont: '700 22px "Noto Serif SC", "Microsoft YaHei", serif',
        bodyFont: '500 14px "Noto Sans SC", "Microsoft YaHei", sans-serif',
        lineHeightTitle: 30,
        lineHeightBody: 22,
        background: "#4a3728",
        accent: "#d2b36f",
        panel: "rgba(80, 55, 38, 0.82)",
        border: "rgba(210, 179, 111, 0.35)",
        title: "#f8f1de",
        body: "rgba(247, 238, 217, 0.88)",
      },
      visual: {
        motif: "watchtower",
        badge: "城市 / 公共频道",
        signalCount: 3,
      },
    };
    if (hudTitleEl) {
      hudTitleEl.textContent = "城邦公共频道";
    }
    renderStageCanvas(canvasEl, defaultVisual);
    canvasEl.dataset.variant = "city";
  }
}

function toggleElements(elements, hidden) {
  for (const element of elements) {
    element.classList.toggle("surface-hidden", hidden);
  }
}

function workspaceViewState() {
  const shellPage = currentShellPage();
  const isUserShell = shellPage === "user";
  const isAdminShell = shellPage === "admin";
  return {
    shellPage,
    isUserShell,
    isAdminShell,
    inlineChatDetail: currentWorkspace === "chat" && isUserShell,
    showChatGovernanceRail: currentWorkspace === "governance",
    worldView: currentWorkspace === "world",
    governanceView: currentWorkspace === "governance",
    userEdgeDrawerVisible: isUserShell,
  };
}

function applyWorkspaceBodyState(viewState) {
  document.body.dataset.workspace = currentWorkspace;
  chatFocusController.syncWithWorkspace();
  document.body.dataset.chatPane = currentWorkspace === "chat" ? chatPaneMode : "split";
  document.body.dataset.chatDetailMode = viewState.inlineChatDetail ? "inline" : "sidebar";
  document.body.dataset.workspaceFocus = currentWorkspace === "chat" ? "chat" : currentWorkspace;
  layoutEl?.classList.toggle("layout-single", currentWorkspace !== "chat");
  layoutEl?.classList.toggle("layout-chat", currentWorkspace === "chat");
  layoutEl?.classList.toggle("layout-chat-inline-detail", viewState.inlineChatDetail);
  document.body.classList.toggle("chat-primary", currentWorkspace === "chat");
}

function syncWorkspaceTabState() {
  for (const button of workspaceTabs) {
    const isActive = button.dataset.workspace === currentWorkspace;
    button.classList.toggle("active", isActive);
    if (isActive) {
      button.setAttribute("aria-current", "page");
    } else {
      button.removeAttribute("aria-current");
    }
  }
}

function applyWorkspacePanelVisibility(viewState) {
  guidePanelEl?.classList.toggle("surface-hidden", currentWorkspace === "chat" && !viewState.isAdminShell);
  governancePanelEl?.classList.toggle(
    "surface-hidden",
    !(
      viewState.userEdgeDrawerVisible ||
      viewState.worldView ||
      viewState.governanceView ||
      viewState.showChatGovernanceRail
    ),
  );
  caretakerPanelEl?.classList.toggle(
    "surface-hidden",
    viewState.isUserShell || currentWorkspace !== "chat",
  );
  authPanelEl?.classList.toggle("surface-hidden", currentWorkspace !== "auth");
  roomsPanelEl?.classList.toggle("surface-hidden", currentWorkspace !== "chat");
  conversationPanelEl?.classList.toggle("surface-hidden", currentWorkspace !== "chat");
  chatDetailPanelEl?.classList.toggle("surface-hidden", currentWorkspace !== "chat" || viewState.inlineChatDetail);
  roomViewToggleButtonEl?.classList.toggle("surface-hidden", currentWorkspace !== "chat");

  toggleElements(governanceBrowseBlocks, !(viewState.worldView || viewState.governanceView || viewState.showChatGovernanceRail));
  toggleElements(worldActionForms, !(viewState.worldView || viewState.showChatGovernanceRail));
  toggleElements(governanceAdminForms, !(viewState.governanceView || viewState.showChatGovernanceRail));
}

function applyWorkspaceChromeEnhancements() {
  chatFocusController.ensureToggle();
  ensureChatPriorityBadge();
  ensureChatQuickLinks();
  updateChatQuickLinksVisibility();
  ensureRoomQuickActions();
  updatePanelTitles();
  ensureConversationCallout();
  updateConversationCallout();
  ensureModeBanner();
  updateModeBanner();
  ensureChatPaneToggle();
}

function applyWorkspace() {
  ensureWorkspaceChrome();
  const viewState = workspaceViewState();
  applyWorkspaceBodyState(viewState);
  syncWorkspaceTabState();
  applyWorkspacePanelVisibility(viewState);
  applyWorkspaceChromeEnhancements();
}

function updatePanelTitles() {
  if (guidePanelTitleEl) {
    guidePanelTitleEl.textContent =
      currentWorkspace === "chat" ? "聊天提示" : "如何开始";
  }
  _updatePanelTitles(shellMode, {
    governanceEl: governancePanelTitleEl,
    authEl: authPanelTitleEl,
    roomsEl: roomsPanelTitleEl,
    conversationEl: conversationPanelTitleEl,
  });
}

function setWorkspace(workspace, { persist = true } = {}) {
  const allowed = availableWorkspacesForShellMode(shellMode);
  currentWorkspace = allowed.includes(workspace)
    ? workspace
    : defaultWorkspaceForShellMode(shellMode);
  if (persist) {
    safeLocalStorageSet(workspaceStorageKey(), currentWorkspace);
  }
  applyWorkspace();
}

function queryGatewayUrl() {
  return gatewayQueryParam(window.location.href);
}

function currentIdentity() {
  return senderIdentity.trim() || "访客";
}

function activeRoom() {
  return activeRoomId ? state.rooms.find((room) => room.id === activeRoomId) : null;
}

function setPersonalRoomAccessPolicyStatus(message = "", isError = false) {
  if (!personalRoomPolicyStatusEl) return;
  personalRoomPolicyStatusEl.textContent = message;
  personalRoomPolicyStatusEl.classList.toggle("is-error", Boolean(isError));
}

function syncPersonalRoomAccessPolicyControl() {
  const controlState = personalRoomAccessPolicyControlState({
    room: activeRoom(),
    currentIdentity: currentIdentity(),
    gatewayUrl,
    saving: personalRoomAccessPolicySaving,
    roomOwnershipForState,
  });

  if (personalRoomPolicyControlEl) {
    personalRoomPolicyControlEl.hidden = controlState.hidden;
    personalRoomPolicyControlEl.setAttribute("aria-hidden", controlState.ariaHidden);
  }

  for (const button of personalRoomPolicyButtons) {
    const buttonPolicy = button.dataset.personalRoomPolicy;
    button.setAttribute("aria-pressed", String(buttonPolicy === controlState.policy));
    button.disabled = controlState.disabled;
  }

  setPersonalRoomAccessPolicyStatus(controlState.statusText, controlState.statusIsError);
}

function applyRailVisibility() {
  // 实现 HTML 中 [data-rail-visibility="owner-only"] 的访问控制：仅已登录居民
  // （Gateway 页面需同时持有 session token）可见，避免默认体验身份或过期身份
  // 进入 scene-editor 等仅 owner 可用的入口。无 Gateway 的静态预览仍允许用
  // identity 查询参数验收已登录居民的视觉状态。
  const hasGatewaySession = !gatewayUrl || Boolean(getSessionToken());
  const isOwner = currentIdentity() !== "访客" && hasGatewaySession;
  document.querySelectorAll('[data-rail-visibility="owner-only"]').forEach((node) => {
    if (isOwner) {
      node.style.removeProperty("display");
    } else {
      node.style.setProperty("display", "none", "important");
    }
  });
  syncPersonalRoomAccessPolicyControl();
}

function residentGatewayLoginRequired() {
  return _residentGatewayLoginRequired(
    userShellProjection(),
    gatewayUrl,
    senderIdentity,
    getSessionToken(),
    allowsSyntheticGatewayIdentity(),
  );
}

function allowsSyntheticGatewayIdentity() {
  if (!gatewayUrl) return false;
  const qaMode = new URLSearchParams(window.location.search).get("qa")?.trim().toLowerCase();
  if (!qaMode || !["browser", "manual"].includes(qaMode)) return false;
  try {
    const host = new URL(gatewayUrl).hostname;
    return ["127.0.0.1", "localhost", "[::1]"].includes(host);
  } catch {
    return false;
  }
}

function gatewayShellStateUrl() {
  return buildGatewayShellStateUrl({
    gatewayUrl,
    residentId: currentIdentity(),
    residentScoped: residentScopedShellStatePage(currentShellPage()),
  });
}

function gatewayShellEventsUrl({ afterVersion = null } = {}) {
  return buildGatewayShellEventsUrl({
    gatewayUrl,
    residentId: currentIdentity(),
    residentScoped: residentScopedShellStatePage(currentShellPage()),
    afterVersion,
  });
}

function applyGovernanceStatusClassState(target, classState) {
  if (!target?.classList) return;
  for (const className of classState.remove) {
    target.classList.remove(className);
  }
  for (const className of classState.add) {
    target.classList.add(className);
  }
}

function setGovernanceStatus(message, isError = false, extraClassName = "") {
  const hasGovernanceStatus = Boolean(governanceStatusEl);
  const target = governanceStatusEl || worldStateEl;
  if (!target) return;
  target.textContent = governanceStatusText({
    message,
    isError,
    shellMode,
    hasGovernanceStatus,
  });
  applyGovernanceStatusClassState(
    target,
    governanceStatusClassState({ isError, extraClassName }),
  );
}

function setAuthStatus(message, isError = false) {
  return setAuthStatusMod(message, isError);
}

function updateResidentLoginSurface() {
  applyResidentLoginSurface(
    userShellProjection(),
    gatewayUrl,
    senderIdentity,
    residentLoginDismissed,
    Boolean(getSessionToken()),
  );
}

function clearChildren(element) {
  while (element.firstChild) {
    element.removeChild(element.firstChild);
  }
}


function buildNodeFromSpec(spec) {
  if (!spec) return null;
  const node = document.createElement(spec.tag || "div");
  if (spec.className) node.className = spec.className;
  if (spec.extraClass) node.classList.add(spec.extraClass);
  if (spec.attrs) {
    for (const [key, value] of Object.entries(spec.attrs)) {
      if (value !== undefined && value !== null) node.setAttribute(key, value);
    }
  }
  if (spec.text !== undefined && spec.text !== null) node.textContent = spec.text;
  if (spec.dataset) {
    for (const [key, value] of Object.entries(spec.dataset)) {
      setDatasetFlag(node, key, value);
    }
  }
  if (Array.isArray(spec.children)) {
    for (const child of spec.children) {
      const childNode = buildNodeFromSpec(child);
      if (childNode) node.appendChild(childNode);
    }
  }
  return node;
}

function createMessageQuickActionChip(action) {
  return buildNodeFromSpec(messageQuickActionChipSpec(action));
}

function createMessageQuickStateChip(action, state = "") {
  return buildNodeFromSpec(messageQuickStateChipSpec(action, state));
}

function createMessageBodyNode(message, options = {}) {
  return buildNodeFromSpec(messageBodyDomSpec(message, options));
}

function roomDisplayPeer(room) { return _roomDisplayPeer(room); }

function roomMemberCount(room) {
  return roomMemberCountForState(room, governanceContextDeps());
}

function roomAudienceLabel(room) {
  return roomAudienceLabelForState(room, governanceContextDeps());
}

function governanceContextDeps() {
  return {
    publicRoomRecordForConversation,
    cityStateForConversation,
    worldDirectoryCity,
    membershipForCity,
    publicRoomsForCity,
    get residents() { return governance?.residents; },
    get world() { return governance?.world; },
    get memberships() { return governance?.memberships; },
    currentIdentity,
    get shellPage() { return currentShellPage(); },
    roomKind,
    roomQuickActionContextCopy,
    roomDisplayPeer,
    roomPreview,
    translateFederationPolicy,
    displayCityTitle,
  };
}

function roomRouteLabel(room) {
  return roomRouteLabelForState(room, governanceContextDeps());
}

function roomSummaryLine(room) {
  return roomSummaryLineForState(room, roomSummaryDeps());
}

function roomStatusLine(room) {
  return roomStatusLineForState(room, roomSummaryDeps());
}

function roomContextSummary(room) {
  return roomContextSummaryForState(room, governanceContextDeps());
}

function roomSummaryDeps() {
  return {
    unreadCount,
    caretakerPendingCount,
    roomHasDraft,
    visiblePendingEchoCount,
    roomSendError: (roomId) => roomSendErrors[roomId],
    latestRoomQuickAction,
    latestRoomQuickState,
    get shellPage() { return currentShellPage(); },
    roomKind,
    roomMemberCount,
    roomQuickActionSummary,
    roomRouteLabel,
    resolveRoomQuickPreview,
    roomQuickPreviewFieldView,
    roomLastActivity,
  };
}

function roomFollowUpCount(room) {
  return roomFollowUpCountForState(room, roomSummaryDeps());
}

function roomChatStatusSummary(room) {
  return roomChatStatusSummaryForState(room, roomSummaryDeps());
}

function roomQueueSummary(room) {
  return roomQueueSummaryForState(room, roomSummaryDeps());
}

function roomThreadHeadline(room) { return _roomThreadHeadline(room); }

function renderConversationMetaChips(room, chips = []) {
  if (!metaEl) return;
  clearChildren(metaEl);
  const title = document.createElement("div");
  title.className = "meta-title";
  title.textContent = room
    ? currentShellPage() === "user"
      ? roomThreadHeadline(room) || room.participant_label || room.title
      : roomThreadHeadline(room)
    : "尚未选择会话";
  metaEl.appendChild(title);
  if (!chips.length) return;
  const row = document.createElement("div");
  row.className = "meta-chip-row";
  for (const { text, tone } of chips) {
    row.appendChild(createMetaChip(text, tone || "muted"));
  }
  metaEl.appendChild(row);
}

function renderRoomDigest(rooms) {
  roomDigestSurfaceRenderer.renderRoomDigest(rooms);
}

function threadStatusRailModel(room, shellPage) {
  const sendError = room ? roomSendErrors[room.id] : "";
  const caretaker = caretakerProfile(room);
  return threadStatusRailModelForState({
    room,
    shellPage,
    threadHeadline: room ? roomThreadHeadline(room) : "",
    chatStatusSummary: room ? roomChatStatusSummary(room) : "",
    queueSummary: room ? roomQueueSummary(room) : "",
    audienceLabel: room ? roomAudienceLabel(room) : "",
    routeLabel: room ? roomRouteLabel(room) : "",
    syncLabel: room ? roomSyncLabel() : "",
    sendError,
    pendingEchoCount: room ? visiblePendingEchoCount(room) : 0,
    caretakerPendingCount: caretakerPendingCount(room),
    unreadCount: room ? unreadCount(room) : 0,
    refreshInProgress: gatewaySyncController.isRefreshing(),
    isSendingMessage: messageSendInFlight(),
    draftLength: room && roomHasDraft(room.id) ? draftForRoom(room.id).trim().length : 0,
    caretaker,
    caretakerStatus: caretakerStatusLine(room),
    caretakerNotificationCount: caretakerNotificationCount(room),
  });
}

function renderThreadStatusRail(room) {
  threadStatusSurfaceRenderer.renderThreadStatusRail(room);
}

function gatewayConnectionStatus() {
  if (!gatewayUrl) return "offline";
  if (gatewayShellStateIsAuthoritative() && !gatewayShellStateAvailable) return "offline";
  const explicitGatewayUrl = Boolean(queryGatewayUrl());
  if (gatewaySyncController.lastErrorMessage() && (explicitGatewayUrl || providerLoaded)) return "offline";
  const providerState = normalizeProviderConnectionState(provider.connection_state);
  if (providerIndicatesGatewayOffline({ providerLoaded, provider, providerState })) return "offline";
  if (
    providerState === "Connecting"
    || (gatewaySyncController.isRefreshing() && !gatewaySyncController.lastSuccessAtMs())
  ) return "connecting";
  if (providerState === "Connected" || gatewaySyncController.lastSuccessAtMs()) return "online";
  return gatewaySyncController.isRefreshing() ? "connecting" : "offline";
}

function translateResidentLabel(residentId) {
  return residentId === currentIdentity() ? "你" : "居民";
}

function gatewayShellStateIsAuthoritative() {
  return Boolean(
    queryGatewayUrl() ||
    safeLocalStorageGet("lobster-gateway-url") ||
    window.location.protocol !== "file:",
  );
}

function roomKind(room) { return _roomKind(room); }

function roomPreview(room) {
  return _roomPreview(room, resolveRoomQuickPreview, latestRoomMessageLike, quickActionPreviewPrimaryFieldText);
}

function roomLastActivity(room) {
  return _roomLastActivity(room, latestRoomMessageLike);
}

function roomActivityTime(room) { return _roomActivityTime(room); }

function badgeToken(value, fallback = "聊") { return _badgeToken(value, fallback); }

// messageAvatarTone / isSystemSender / messageThreadKind / messageRoleLabel moved to shell-message-render.js

function roomSyncLabel() {
  if (gatewaySyncController.isRefreshing()) return "同步中";
  const lastSuccessAtMs = gatewaySyncController.lastSuccessAtMs();
  if (!lastSuccessAtMs) return gatewayUrl ? "尚未同步" : "离线";
  return `最近同步 ${new Date(lastSuccessAtMs).toLocaleTimeString()}`;
}


function filteredRooms() { return _filteredRooms(state.rooms, roomFilter, roomSearch); }

function updateRoomToolbarState() {
  const shellPage = currentShellPage();
  for (const button of roomFilterButtons) {
    if (button.dataset.roomFilter === "all") {
      button.textContent = shellPage === "admin" ? "全部会话" : "全部";
    } else if (button.dataset.roomFilter === "direct") {
      button.textContent = "私信";
    } else if (button.dataset.roomFilter === "public") {
      button.textContent = "群聊";
    }
    button.classList.toggle("active", button.dataset.roomFilter === roomFilter);
  }
  if (roomSearchInputEl) {
    roomSearchInputEl.placeholder =
      shellPage === "user"
        ? "搜索居民、房间或最近消息"
        : shellPage === "admin"
        ? "搜索会话、频道、访客提醒或最近消息"
        : "搜索会话、私信、群聊或最近消息";
  }
  if (roomSearchInputEl && roomSearchInputEl.value.toLowerCase() !== roomSearch) {
    roomSearchInputEl.value = roomSearch;
  }
}

function focusRoom(roomId) {
  const shouldFollowExistingTimeline =
    !timelineEl || timelineEl.scrollHeight - timelineEl.scrollTop - timelineEl.clientHeight < 80;
  if (activeRoomId && activeRoomId !== roomId && timelineEl) {
    timelineEl.setAttribute("data-switching", "true");
    requestAnimationFrame(() => {
      setTimeout(() => {
        if (timelineEl) timelineEl.removeAttribute("data-switching");
      }, 160);
    });
  }
  activeRoomId = roomId;
  if (editingMessageTarget?.roomId && editingMessageTarget.roomId !== roomId) {
    clearMessageEditTarget({ clearInput: true });
  }
  roomSearch = "";
  roomFilter = "all";
  searchMode = "all";
  if (searchModeControlsEl) updateSearchModeTabs();
  followTimelineToLatest = shouldFollowExistingTimeline;
  syncComposerDraft({ force: true });
  syncChatPaneMode(window.matchMedia("(max-width: 960px)").matches ? "thread" : "split");
  markRoomRead(roomId);
  updateRoomToolbarState();
  syncPersonalRoomAccessPolicyControl();
  setWorkspace("chat");
  updateCaretakerStatus();
  renderConversationOverview();
  updateComposerState();
  focusComposerInput({ force: true });
  // Close mobile drawers after selecting a room
  railDrawerEl?.classList.remove("open");
  setSfcRailOpen(false);
}

/**
 * Returns the online status of the peer in a direct conversation room,
 * by cross-referencing governance.residents with the room's participant list.
 * @returns {"online" | "offline" | null}
 */
function directRoomPeerOnlineStatus(room) {
  return directRoomPeerOnlineStatusForState(room, governanceContextDeps());
}

function confirmResidentRoomJump(room) {
  if (!room || room.id === activeRoomId) return false;
  const target = roomThreadHeadline(room);
  const message = `进入「${target}」的房间私聊？`;
  if (typeof window.confirm === "function" && !window.confirm(message)) {
    return false;
  }
  focusRoom(room.id);
  renderRooms();
  renderTimeline();
  return true;
}

// formatDateTime moved to shell-message-render.js
// joinOrFallback moved to shell-payload.js

function localPreviewMessagesForEmptyRoom(room) {
  return buildLocalPreviewMessagesForEmptyRoom({
    room,
    gatewayUrl,
    shellPage: currentShellPage(),
    shellVariant: document.body?.dataset?.shellVariant || "",
    currentIdentity: currentIdentity(),
  });
}

function actorIsWorldSteward() {
  const stewards = governance.world_safety?.stewards || [];
  return stewards.includes(currentIdentity());
}

function membershipForCity(cityId) {
  return governance.memberships.find(
    (membership) =>
      membership.city_id === cityId && membership.resident_id === currentIdentity(),
  );
}

function publicRoomsForCity(cityId) {
  return governance.public_rooms.filter((room) => room.city_id === cityId);
}

function publicRoomRecordForConversation(roomId) {
  return governance.public_rooms.find((room) => room.room_id === roomId) || null;
}

function cityStateForConversation(roomId) {
  const room = publicRoomRecordForConversation(roomId);
  if (!room) return null;
  return governance.cities.find((item) => item.profile.city_id === room.city_id) || null;
}

function worldDirectoryCity(cityId) {
  return governance.world_directory?.cities?.find((city) => city.city_id === cityId) || null;
}

// humanMembership, hasConversationShellPayload, hasAnyShellPayload, normalizeShellMessages
// moved to shell-payload.js
async function loadBootstrap() {
  try {
    const candidates = ["./generated/bootstrap.json", "./bootstrap.sample.json"];
    for (const url of candidates) {
      const response = await fetch(url);
      if (!response.ok) continue;
      bootstrap = await response.json();
      return;
    }
  } catch {
    // fall through
  }
  bootstrap = DEFAULT_BOOTSTRAP;
}

async function loadGatewayBootstrap() {
  if (!gatewayUrl) return;
  try {
    const response = await fetch(`${gatewayUrl}/v1/shell/bootstrap`);
    if (!response.ok) return;
    bootstrap = await response.json();
  } catch {
    // keep prior bootstrap
  }
}

async function loadShellState() {
  try {
    const candidates = ["./generated/state.json"];
    for (const url of candidates) {
      const response = await fetch(url);
      if (!response.ok) continue;
      const payload = await response.json();
      if (hasAnyShellPayload(payload)) {
        state = normalizeShellStateForState(payload, SAMPLE_STATE);
        activeRoomId = defaultActiveRoomId(state.rooms) ?? activeRoomId;
        syncComposerDraft({ force: true });
        return;
      }
    }
  } catch {
    // keep fallback sample
  }
}

function clearGatewayShellState() {
  state = structuredClone(GATEWAY_EMPTY_STATE);
  activeRoomId = null;
  lastShellStateVersion = null;
  gatewayShellStateAvailable = false;
  syncComposerDraft({ force: true });
  return false;
}

async function loadGatewayState() {
  if (!gatewayUrl) return false;
  try {
    const shellStateUrl = gatewayShellStateUrl();
    // /v1/shell/state 对居民视角要求 Bearer session(无 session 401);
    // 与 postGatewayJson 一致携带 session token,否则轮询永远静默失败。
    const response = await fetch(shellStateUrl, {
      headers: gatewayJsonHeaders(getSessionToken()),
    });
    if (!response.ok) {
      handleGatewayAuthFailure(response.status);
      return gatewayShellStateIsAuthoritative() ? clearGatewayShellState() : false;
    }
    const payload = await response.json();
    return applyGatewayShellStatePayload(payload, { persist: true });
  } catch {
    return gatewayShellStateIsAuthoritative() ? clearGatewayShellState() : false;
  }
}

async function applyGatewayShellStatePayload(payload, { persist = false } = {}) {
  if (!hasGatewayShellStatePayload(payload)) {
    return gatewayShellStateIsAuthoritative() ? clearGatewayShellState() : false;
  }
  state = normalizeShellStateForState(payload, GATEWAY_EMPTY_STATE);
  gatewayShellStateAvailable = true;
  const nextActiveRoomId = state.rooms.some((room) => room.id === activeRoomId)
    ? activeRoomId
    : defaultActiveRoomId(state.rooms);
  const activeChanged = nextActiveRoomId !== activeRoomId;
  activeRoomId = nextActiveRoomId;
  if (activeChanged) {
    syncComposerDraft({ force: true });
  }
  if (persist) {
    await persistState();
  }
  return true;
}

async function loadWorldState() {
  if (!gatewayUrl) return false;
  try {
    const residentsUrl = new URL(`${gatewayUrl}/v1/residents`);
    if (!isVisitorIdentity(currentIdentity())) {
      residentsUrl.searchParams.set("resident_id", currentIdentity());
    }
    const residentsRequest = fetch(residentsUrl.toString());
    const snapshotResponse = await fetch(`${gatewayUrl}/v1/world-snapshot`);
    if (snapshotResponse.ok) {
      const bundle = await snapshotResponse.json();
      const snapshotGovernance = governanceFromWorldSnapshotBundle(bundle);
      if (snapshotGovernance) {
        const residentsResponse = await residentsRequest;
        const scopedResidentsPayload = residentsResponse.ok
          ? await residentsResponse.json()
          : snapshotGovernance.residents;
        const scopedGovernance = governanceWithResidentsPayload(snapshotGovernance, scopedResidentsPayload);
        governance = scopedGovernance;
        return true;
      }
    }

    const [worldResponse, residentsResponse] = await Promise.all([
      fetch(`${gatewayUrl}/v1/world`),
      residentsRequest,
    ]);
    if (!worldResponse.ok) return false;
    const payload = await worldResponse.json();
    const residentsPayload = residentsResponse.ok ? await residentsResponse.json() : [];
    const apiGovernance = governanceFromWorldApiPayload(payload, residentsPayload);
    if (apiGovernance) {
      governance = apiGovernance;
      return true;
    }
  } catch {
    // keep last governance snapshot
  }
  return false;
}

async function loadProviderState() {
  if (!gatewayUrl) return false;
  try {
    const response = await fetch(`${gatewayUrl}/v1/provider`);
    if (!response.ok) return false;
    const payload = await response.json();
    if (payload?.mode) {
      provider = payload;
      providerLoaded = true;
      return true;
    }
  } catch {
    // keep prior provider snapshot
  }
  return false;
}

function worldEntryRouteListElement() {
  return document.querySelector(".world-route-list");
}

async function fetchWorldEntryPayload() {
  const response = await fetch(`${gatewayUrl}/v1/world-entry`);
  if (!response.ok) return null;
  const payload = await response.json();
  const routes = Array.isArray(payload?.routes) ? payload.routes : [];
  if (routes.length === 0) return null;
  return { ...payload, routes };
}

function syncWorldEntryHud(payload) {
  const hudTitle = document.querySelector(".world-entry-hud .hud-title");
  const stationChip = document.querySelector(".world-entry-hud-chip");
  const hudStatus = document.querySelector("#hud-status");
  if (hudTitle && payload.title) {
    hudTitle.textContent = payload.title;
  }
  if (stationChip && payload.station_label) {
    stationChip.textContent = payload.station_label;
  }
  if (hudStatus && payload.source_summary) {
    hudStatus.textContent = payload.source_summary;
  }
}

function createWorldSquareRouteOptionNode() {
  const option = document.createElement("a");
  option.className = "world-route-option world-route-option-square";
  option.setAttribute("href", "./world-square.html");

  const title = document.createElement("strong");
  title.textContent = "世界广场";
  option.appendChild(title);

  const desc = document.createElement("span");
  desc.textContent = "打开之前绘制的世界广场完整素材，作为公共广场入口。";
  option.appendChild(desc);

  const status = document.createElement("span");
  status.className = "world-route-status";
  status.textContent = "概念图 · 公共广场";
  option.appendChild(status);
  return option;
}

function createWorldEntryRouteOptionNode(route) {
  const option = document.createElement("a");
  option.className = "world-route-option";
  if (route.is_current) {
    option.classList.add("is-current");
  }
  option.setAttribute("href", route.href || "#");

  const title = document.createElement("strong");
  title.textContent = route.title || "";
  option.appendChild(title);

  if (route.description) {
    const desc = document.createElement("span");
    desc.textContent = route.description;
    option.appendChild(desc);
  }

  if (route.status_label) {
    const status = document.createElement("span");
    status.className = "world-route-status";
    status.textContent = route.is_current ? `当前主城 · ${route.status_label}` : route.status_label;
    option.appendChild(status);
  } else if (route.is_current) {
    const status = document.createElement("span");
    status.className = "world-route-status";
    status.textContent = "当前主城";
    option.appendChild(status);
  }
  return option;
}

function renderWorldEntryRoutes(routeList, routes) {
  routeList.replaceChildren();
  routeList.appendChild(createWorldSquareRouteOptionNode());
  for (const route of routes) {
    routeList.appendChild(createWorldEntryRouteOptionNode(route));
  }
}

async function loadWorldEntry() {
  const shellPage = currentShellPage();
  if (shellPage !== "world-entry") return false;
  const routeList = worldEntryRouteListElement();
  if (!routeList) return false;
  if (!gatewayUrl) return false;

  try {
    const payload = await fetchWorldEntryPayload();
    if (!payload) return false;
    syncWorldEntryHud(payload);
    renderWorldEntryRoutes(routeList, payload.routes);
    return true;
  } catch {
    return false;
  }
}

function openIndexedDb() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open("lobster-chat-shell", 1);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains("shell")) {
        db.createObjectStore("shell");
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function loadCachedState() {
  if (!("indexedDB" in window)) {
    setNodeText(storageStateEl, "存储：内存回退模式");
    return;
  }

  try {
    const db = await openIndexedDb();
    const tx = db.transaction("shell", "readonly");
    const store = tx.objectStore("shell");
    const cached = await new Promise((resolve, reject) => {
      const req = store.get("timeline-state");
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
    if (hasAnyShellPayload(cached)) {
      state = normalizeShellStateForState(cached, SAMPLE_STATE);
    }
    setNodeText(storageStateEl, "存储：本地数据库已就绪");
  } catch {
    setNodeText(storageStateEl, "存储：回退模式");
  }
}

async function persistState() {
  if (!("indexedDB" in window)) return;

  try {
    const db = await openIndexedDb();
    const tx = db.transaction("shell", "readwrite");
    tx.objectStore("shell").put(state, "timeline-state");
  } catch {
    // keep shell quiet in fallback mode
  }
}

function loadSenderIdentity() {
  const queryIdentity = new URLSearchParams(window.location.search).get("identity")?.trim();
  const syntheticIdentity = allowsSyntheticGatewayIdentity();
  if (queryIdentity && (!gatewayUrl || syntheticIdentity)) {
    senderIdentity = queryIdentity;
  } else if (gatewayUrl && !getSessionToken()) {
    senderIdentity = "访客";
  } else {
    const stored = safeLocalStorageGet("lobster-identity");
    if (stored?.trim()) {
      senderIdentity = stored.trim();
    } else {
      const preset = defaultIdentityForShellMode(shellMode);
      if (preset) {
        senderIdentity = preset;
      }
    }
  }
  if (identityInputEl) {
    identityInputEl.value = senderIdentity;
  }
  applyRailVisibility();
}

function persistSenderIdentity(value) {
  const nextIdentity = value.trim() || "访客";
  const identityChanged = nextIdentity !== senderIdentity;
  senderIdentity = nextIdentity;
  safeLocalStorageSet("lobster-identity", senderIdentity);
  if (identityInputEl && identityInputEl.value !== senderIdentity) {
    identityInputEl.value = senderIdentity;
  }
  if (identityChanged) {
    clearAllPendingEchoes();
    roomSendErrors = {};
  }
  updateResidentLoginSurface();
  applyRailVisibility();
}

async function refreshIdentityProjection() {
  renderGovernance();
  renderResidents();
  renderRooms();
  renderTimeline();
  updateComposerState();
  updateGovernanceFormState();
  if (!gatewayUrl) {
    return;
  }
  await loadGatewayState();
  await loadWorldState();
  renderGovernance();
  renderResidents();
  renderRooms();
  renderTimeline();
  updateComposerState();
  updateGovernanceFormState();
}

function loadAuthDraft() {
  return loadAuthDraftMod();
}

function persistAuthDraft() {
  return persistAuthDraftMod();
}

function renderRooms() {
  roomListSurfaceRenderer.renderRooms();
}

function conversationOverviewHeaderModelForRoom(room, shellPage, compactChatShell) {
  return conversationOverviewHeaderModel({
    shellPage,
    threadHeadline: roomThreadHeadline(room),
    summaryLine: roomSummaryLine(room),
    overviewSummary: room.overview_summary,
    contextSummary: room.context_summary,
    subtitle: room.subtitle,
    roomKind: roomKind(room),
    roomKindLabel: translateRoomKind(roomKind(room)),
    audienceLabel: roomAudienceLabel(room),
    identity: currentIdentity(),
    compactChatShell,
    sceneBanner: room.scene_banner,
    caretaker: caretakerProfile(room),
  });
}

function createConversationOverviewHeaderNode(room, shellPage, compactChatShell) {
  const model = conversationOverviewHeaderModelForRoom(room, shellPage, compactChatShell);
  const header = document.createElement("div");
  header.className = "overview-header";

  const titleWrap = document.createElement("div");
  titleWrap.className = "overview-title-wrap";
  titleWrap.appendChild(createLine("overview-title", model.title));
  titleWrap.appendChild(createLine("overview-summary", model.summary));
  header.appendChild(titleWrap);

  const badgeWrap = document.createElement("div");
  badgeWrap.className = "overview-meta";
  for (const pill of model.pills) {
    badgeWrap.appendChild(createPill(pill.text, pill.tone));
  }
  header.appendChild(badgeWrap);
  return header;
}

function appendUserConversationQuickPreview(room, preview) {
  if (!preview) return;
  const previewFieldView = roomQuickPreviewFieldView(
    room.id,
    preview.action,
    preview.state,
    preview.snapshotIndex,
  );
  const previewSummaryLine = createQuickActionPreviewSummaryLine(preview, {
    className: "overview-summary overview-summary-preview quick-action-preview-summary",
    includePrefix: true,
    fieldView: previewFieldView,
  });
  if (previewSummaryLine) {
    conversationOverviewEl.appendChild(previewSummaryLine);
  }
  const previewCard = createQuickActionPreviewCard(preview.action, preview.state, preview.structured, {
    className: "overview-preview-card",
    maxFields: 3,
    roomId: room.id,
    fieldView: previewFieldView,
    historyLabel: preview.historyLabel || "",
    history: preview.history,
    selectedHistoryIndex: preview.snapshotIndex,
    onHistoryClick: (_snapshot, index) => {
      previewRoomQuickStage(room.id, preview.action, preview.state, index);
    },
    onFieldViewChange: (viewId) => {
      setRoomQuickPreviewFieldView(room.id, preview.action, preview.state, preview.snapshotIndex, viewId);
    },
  });
  if (previewCard) {
    conversationOverviewEl.appendChild(previewCard);
  }
}

function createUserConversationStatusNode(room) {
  const model = userConversationStatusPillsForRoom(room);
  const userStatus = document.createElement("div");
  userStatus.className = "overview-status";
  for (const pill of model.leadingPills) {
    userStatus.appendChild(createPill(pill.text, pill.tone));
  }
  const roomActionPill = createRoomQuickActionPill(room);
  if (roomActionPill) {
    userStatus.appendChild(roomActionPill);
  }
  for (const pill of model.trailingPills) {
    userStatus.appendChild(createPill(pill.text, pill.tone));
  }
  return userStatus;
}

function userConversationStatusPillsForRoom(room) {
  return userConversationStatusPills({
    syncLabel: roomSyncLabel(),
    refreshInProgress: gatewaySyncController.isRefreshing(),
    unreadCount: unreadCount(room),
    caretaker: caretakerProfile(room),
    caretakerPendingCount: caretakerPendingCount(room),
    hasDraft: roomHasDraft(room.id),
    hasSendError: Boolean(roomSendErrors[room.id]),
    isSendingMessage: messageSendInFlight(),
  });
}

function createUserConversationWorkflowNode(room) {
  return createWorkflowProgress(latestRoomQuickAction(room), latestRoomQuickState(room), {
    className: "overview-workflow-progress",
    title: "当前阶段",
    stages: workflowProfile(room)?.steps,
    onStageClick: (stage) => {
      const action = latestRoomQuickAction(room);
      if (!action) return;
      previewRoomQuickStage(room.id, action, stage.label);
      seedComposerFromQuickAction(action, quickActionWorkflowTemplate(action, stage.label), { force: true });
    },
  });
}

function createUserConversationActionsNode(room) {
  const userActions = document.createElement("div");
  userActions.className = "overview-actions";

  const refreshButton = document.createElement("button");
  refreshButton.type = "button";
  refreshButton.textContent = "刷新聊天";
  refreshButton.disabled = !gatewayUrl;
  refreshButton.addEventListener("click", async () => {
    if (!gatewayUrl) return;
    await refreshFromGateway();
  });
  userActions.appendChild(refreshButton);
  appendRoomQuickActionOverviewButton(userActions, room);
  appendRoomQuickStateAdvanceButton(userActions, room);
  return userActions;
}

function syncRoomViewToggleButton() {
  if (roomViewToggleButtonEl) {
    roomViewToggleButtonEl.textContent = chatPaneMode === "list" ? "返回会话" : "会话列表";
  }
}

function appendUserConversationOverview(room) {
  const previewAction = latestRoomQuickAction(room);
  const preview = resolveRoomQuickPreview(room, previewAction);
  conversationOverviewEl.appendChild(createLine("overview-summary", roomOverviewSummary(room)));
  appendUserConversationQuickPreview(room, preview);
  conversationOverviewEl.appendChild(createUserConversationStatusNode(room));
  const userWorkflow = createUserConversationWorkflowNode(room);
  if (userWorkflow) {
    conversationOverviewEl.appendChild(userWorkflow);
  }
  conversationOverviewEl.appendChild(createUserConversationActionsNode(room));
  syncRoomViewToggleButton();
  updateConversationCallout();
}

function createConversationOverviewContextNode(room, shellPage) {
  const model = conversationOverviewContextModelForRoom(room, shellPage);
  const context = document.createElement("div");
  context.className = "overview-context";
  context.appendChild(createLine("overview-context-title", model.title));
  for (const copy of model.copies) {
    context.appendChild(createLine("overview-context-copy", copy));
  }
  return context;
}

function conversationOverviewContextModelForRoom(room, shellPage) {
  return conversationOverviewContextModel({
    shellPage,
    summaryLine: roomSummaryLine(room),
    contextSummary: roomContextSummary(room),
    statusLine: roomStatusLine(room),
  });
}

function createConversationOverviewStatusNode(room, shellPage, compactChatShell) {
  const status = document.createElement("div");
  status.className = "overview-status";
  appendConversationOverviewBaseStatusPills(status, room, compactChatShell);
  appendConversationOverviewRoomStatePills(status, room);
  appendConversationOverviewCaretakerStatusPill(status, room);
  appendConversationOverviewRuntimeStatusPills(status, room);
  return status;
}

function appendConversationOverviewBaseStatusPills(status, room, compactChatShell) {
  const pills = conversationOverviewBaseStatusPillsForRoom(room, compactChatShell);
  for (const pill of pills) {
    status.appendChild(createPill(pill.text, pill.tone));
  }
}

function conversationOverviewBaseStatusPillsForRoom(room, compactChatShell) {
  return conversationOverviewBaseStatusPills({
    chatStatusSummary: roomChatStatusSummary(room),
    queueSummary: roomQueueSummary(room),
    syncLabel: roomSyncLabel(),
    routeLabel: roomRouteLabel(room),
    hasSendError: Boolean(roomSendErrors[room.id]),
    pendingEchoCount: visiblePendingEchoCount(room),
    caretakerPendingCount: caretakerPendingCount(room),
    unreadCount: unreadCount(room),
    refreshInProgress: gatewaySyncController.isRefreshing(),
    compactChatShell,
    messageCount: room.messages?.length || 0,
  });
}

function appendConversationOverviewRoomStatePills(status, room) {
  const roomActionPill = createRoomQuickActionPill(room);
  if (roomActionPill) {
    status.appendChild(roomActionPill);
  }
  const draftPill = conversationOverviewDraftPill({
    hasDraft: roomHasDraft(room.id),
    draftLength: draftForRoom(room.id).trim().length,
  });
  if (draftPill) {
    status.appendChild(createPill(draftPill.text, draftPill.tone));
  }
}

function appendConversationOverviewCaretakerStatusPill(status, room) {
  const caretakerPill = conversationOverviewCaretakerStatusPillModel({
    caretaker: caretakerProfile(room),
    caretakerPendingCount: caretakerPendingCount(room),
  });
  if (caretakerPill) {
    status.appendChild(createPill(caretakerPill.text, caretakerPill.tone));
  }
}

function appendConversationOverviewRuntimeStatusPills(status, room) {
  const pills = conversationOverviewRuntimeStatusPills({
    isSendingMessage: messageSendInFlight(),
    hasSendError: Boolean(roomSendErrors[room.id]),
    hasSyncFallback: Boolean(gatewaySyncController.lastErrorMessage()),
  });
  for (const pill of pills) {
    status.appendChild(createPill(pill.text, pill.tone));
  }
}

function createConversationOverviewRefreshButton(shellPage) {
  const refreshButton = document.createElement("button");
  refreshButton.type = "button";
  refreshButton.textContent = shellPage === "admin" ? "刷新会话" : "刷新聊天";
  refreshButton.disabled = !gatewayUrl;
  refreshButton.addEventListener("click", async () => {
    if (!gatewayUrl) return;
    await refreshFromGateway();
  });
  return refreshButton;
}

function createConversationOverviewExportButton(shellPage) {
  const exportButton = document.createElement("button");
  exportButton.type = "button";
  exportButton.textContent = shellPage === "admin" ? "导出会话" : "导出聊天";
  exportButton.disabled = !gatewayUrl || !activeRoomId;
  exportButton.addEventListener("click", () => {
    void exportCurrentConversation(shellPage === "admin" ? "导出会话失败" : "导出聊天失败");
  });
  return exportButton;
}

function createConversationOverviewWorldButton(room, shellPage) {
  const worldButton = document.createElement("button");
  worldButton.type = "button";
  worldButton.textContent =
    shellPage === "admin"
      ? roomKind(room) === "direct"
        ? "转到频道"
        : "去找居民"
      : roomKind(room) === "direct"
        ? "去找频道"
        : "去找人";
  worldButton.addEventListener("click", () => setWorkspace("world"));
  return worldButton;
}

function appendConversationOverviewNavigationButtons(actions, room, shellPage) {
  if (shellPage !== "user") {
    actions.appendChild(createConversationOverviewWorldButton(room, shellPage));
  }

  if (shellMode !== "user") {
    const governanceButton = document.createElement("button");
    governanceButton.type = "button";
    governanceButton.textContent = "更多";
    governanceButton.addEventListener("click", () => setWorkspace("governance"));
    actions.appendChild(governanceButton);
  }
}

function createConversationOverviewActionsNode(room, shellPage) {
  const actions = document.createElement("div");
  actions.className = "overview-actions";

  actions.appendChild(createConversationOverviewRefreshButton(shellPage));
  actions.appendChild(createConversationOverviewExportButton(shellPage));
  appendRoomQuickActionOverviewButton(actions, room);
  appendRoomQuickStateAdvanceButton(actions, room);
  appendConversationOverviewNavigationButtons(actions, room, shellPage);

  return actions;
}

function renderConversationOverviewEmptyState() {
  conversationOverviewEl.appendChild(createLine("overview-title", "还没有打开聊天"));
  conversationOverviewEl.appendChild(
    createLine(
      "overview-summary",
      gatewayUrl
        ? "先去群聊页打开一个群聊，或者直接发起私信，聊天区就会进入可发送状态。"
        : "当前在离线预览态，只显示样例会话；连接网关后会接入真实消息流。",
    ),
  );
  updateConversationCallout();
}

function appendNonUserConversationOverview(room, shellPage, compactChatShell) {
  conversationOverviewEl.appendChild(
    createLine(
      "overview-summary",
      shellPage === "admin" ? `当前窗口重点：${roomOverviewSummary(room)}` : roomOverviewSummary(room),
    ),
  );

  const context = createConversationOverviewContextNode(room, shellPage);
  conversationOverviewEl.appendChild(context);

  const status = createConversationOverviewStatusNode(room, shellPage, compactChatShell);
  conversationOverviewEl.appendChild(status);

  const actions = createConversationOverviewActionsNode(room, shellPage);
  conversationOverviewEl.appendChild(actions);

  if (roomViewToggleButtonEl) {
    roomViewToggleButtonEl.textContent = chatPaneMode === "list" ? "返回会话" : "会话列表";
  }
  updateConversationCallout();
}

function renderConversationOverview() {
  if (!conversationOverviewEl) return;
  clearChildren(conversationOverviewEl);
  const shellPage = currentShellPage();
  const compactChatShell = shellPage === "user" || shellPage === "admin";

  const room = state.rooms.find((item) => item.id === activeRoomId);
  if (!room) {
    renderConversationOverviewEmptyState();
    return;
  }

  const header = createConversationOverviewHeaderNode(room, shellPage, compactChatShell);
  conversationOverviewEl.appendChild(header);

  if (shellPage === "user") {
    appendUserConversationOverview(room);
    return;
  }

  appendNonUserConversationOverview(room, shellPage, compactChatShell);
}

function createChatDetailHeroNode(room, shellPage) {
  const hero = document.createElement("section");
  hero.className = "chat-detail-hero";
  hero.appendChild(createLine("chat-detail-title", roomThreadHeadline(room)));
  hero.appendChild(createLine("chat-detail-copy", roomContextSummary(room)));

  const pills = document.createElement("div");
  pills.className = "chat-detail-pills";
  pills.appendChild(
    createPill(
      translateRoomKindForShellPage(roomKind(room), shellPage),
      roomKind(room) === "direct" ? "accent" : "warm",
    ),
  );
  pills.appendChild(createPill(roomAudienceLabel(room), "muted"));
  pills.appendChild(createPill(`身份 ${currentIdentity()}`, "muted"));
  if (unreadCount(room) > 0) {
    pills.appendChild(createPill(`${unreadCount(room)} 条未读`, "warm"));
  }
  if (roomHasDraft(room.id)) {
    pills.appendChild(createPill("草稿未发", "accent"));
  }
  if (room.scene_banner) {
    pills.appendChild(createPill(room.scene_banner, "muted"));
  }
  if (caretakerProfile(room)) {
    pills.appendChild(createPill(`${caretakerProfile(room).name} 在岗`, "accent"));
  }
  hero.appendChild(pills);
  return hero;
}

function chatRuntimeQuickActionContext(room) {
  const latestAction = latestRoomQuickAction(room);
  if (!latestAction) return null;
  return {
    latestAction,
    quickState: latestRoomQuickState(room),
    preview: resolveRoomQuickPreview(room, latestAction),
  };
}

function chatRuntimePreviewFieldView(room, preview) {
  return roomQuickPreviewFieldView(
    room.id,
    preview.action,
    preview.state,
    preview.snapshotIndex,
  );
}

function createChatRuntimePreviewSummaryRow(preview, previewFieldView) {
  const previewSummaryLine = createQuickActionPreviewSummaryLine(preview, {
    tagName: "span",
    className: "quick-action-preview-summary-line",
    fieldView: previewFieldView,
  });
  return createDetailRow("阶段预览", previewSummaryLine || preview.detailText);
}

function createChatRuntimePreviewCardNode(room, preview, previewFieldView) {
  return createQuickActionPreviewCard(preview.action, preview.state, preview.structured, {
    className: "chat-detail-preview-card",
    maxFields: 3,
    roomId: room.id,
    fieldView: previewFieldView,
    historyLabel: preview.historyLabel || "",
    history: preview.history,
    selectedHistoryIndex: preview.snapshotIndex,
    onHistoryClick: (_snapshot, index) => {
      previewRoomQuickStage(room.id, preview.action, preview.state, index);
    },
    onFieldViewChange: (viewId) => {
      setRoomQuickPreviewFieldView(room.id, preview.action, preview.state, preview.snapshotIndex, viewId);
    },
  });
}

function appendChatRuntimePreviewRows(runtime, room, preview) {
  if (!preview) return;
  const previewFieldView = chatRuntimePreviewFieldView(room, preview);
  runtime.appendChild(createChatRuntimePreviewSummaryRow(preview, previewFieldView));
  const previewCard = createChatRuntimePreviewCardNode(room, preview, previewFieldView);
  if (previewCard) {
    runtime.appendChild(previewCard);
  }
}

function chatRuntimeDetailModel(room, shellPage) {
  const caretaker = caretakerProfile(room);
  return chatRuntimeDetailModelForState({
    room,
    shellPage,
    threadHeadline: roomThreadHeadline(room),
    chatStatusSummary: roomChatStatusSummary(room),
    queueSummary: roomQueueSummary(room),
    syncLabel: roomSyncLabel(),
    quickContext: chatRuntimeQuickActionContext(room),
    provider,
    gatewayUrl,
    isSendingMessage: messageSendInFlight(),
    caretakerStatus: caretaker ? caretakerStatusLine(room) : "",
    sendError: roomSendErrors[room.id] || "",
  });
}

function appendChatRuntimeRows(runtime, rows) {
  for (const row of rows) {
    runtime.appendChild(createDetailRow(row.label, row.value));
  }
}

function createChatRuntimeDetailSection(room, shellPage) {
  const runtime = createDetailSection("聊天状态");
  const model = chatRuntimeDetailModel(room, shellPage);
  appendChatRuntimeRows(runtime, model.rowsBeforePreview);
  appendChatRuntimePreviewRows(runtime, room, model.preview);
  appendChatRuntimeRows(runtime, model.rowsAfterPreview);
  return runtime;
}

function chatDetailRoomContextModel(room) {
  return chatDetailRoomContextModelForState(room, governanceContextDeps());
}

function createChatDetailCityContextSection(context) {
  const { publicRoom, cityState, directoryCity, membership, cityProfile } = context;
  const citySection = createDetailSection(
    "城市 / 频道资料",
    publicRoom.description || displayCityDescription(cityProfile),
  );
  citySection.appendChild(createDetailRow("城市", displayCityTitle(cityProfile)));
  citySection.appendChild(createDetailRow("频道", publicRoom.slug || publicRoom.room_id));
  citySection.appendChild(
    createDetailRow("治理状态", publicRoom.frozen ? "房间已冻结" : "房间可发言"),
  );
  if (directoryCity?.trust_state) {
    citySection.appendChild(
      createDetailRow("世界信任", translateTrustState(directoryCity.trust_state)),
    );
  }
  if (cityState?.profile?.federation_policy) {
    citySection.appendChild(
      createDetailRow(
        "联邦策略",
        translateFederationPolicy(cityState.profile.federation_policy),
      ),
    );
  }
  if (membership) {
    citySection.appendChild(createDetailRow("你的身份", humanMembership(membership)));
  }
  return citySection;
}

function createChatDetailSiblingRoomsSection(context) {
  if (!context.siblingRooms.length) return null;
  const related = createDetailSection("同城其他群聊");
  const list = document.createElement("div");
  list.className = "chat-detail-list";
  for (const sibling of context.siblingRooms.slice(0, 5)) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "chat-detail-link";
    button.textContent = `${sibling.slug} · ${sibling.frozen ? "已冻结" : "可发言"}`;
    button.addEventListener("click", async () => {
      focusRoom(sibling.room_id);
      await loadGatewayState();
      renderRooms();
      renderTimeline();
    });
    list.appendChild(button);
  }
  related.appendChild(list);
  return related;
}

function createChatDetailDirectContextSection(room) {
  const direct = createDetailSection(
    "私信窗口",
    governance.world?.allows_cross_city_private_messages
      ? "当前世界允许跨城私信，适合直接协作、追问和一对一沟通。"
      : "当前世界未开启跨城私信，建议优先在同城身份下沟通。",
  );
  direct.appendChild(
    createDetailRow(
      "会话对象",
      room.peer_label || room.participant_label || roomAudienceLabel(room) || roomDisplayPeer(room) || "私信对象",
    ),
  );
  direct.appendChild(
    createDetailRow(
      "跨城私信",
      governance.world?.allows_cross_city_private_messages ? "已开启" : "已关闭",
    ),
  );
  direct.appendChild(createDetailRow("窗口类型", "点对点聊天"));
  return direct;
}

function appendChatDetailRoomContextSections(container, room) {
  const context = chatDetailRoomContextModel(room);
  if (context.publicRoom) {
    container.appendChild(createChatDetailCityContextSection(context));
    const siblingSection = createChatDetailSiblingRoomsSection(context);
    if (siblingSection) {
      container.appendChild(siblingSection);
    }
    return;
  }
  container.appendChild(createChatDetailDirectContextSection(room));
}

function createCaretakerDetailSection(room) {
  const caretaker = caretakerProfile(room);
  if (!caretaker) return null;
  const caretakerSection = createDetailSection(
    `${caretaker.role_label || "房间管家"} · ${caretaker.name}`,
    caretaker.persona || "这只小狗会帮主人记住访客、留言和需要提醒的事情。",
  );
  caretakerSection.appendChild(createDetailRow("人设", caretaker.persona || "未设定"));
  caretakerSection.appendChild(createDetailRow("短期记忆", caretaker.memory || "暂无记录"));
  caretakerSection.appendChild(createDetailRow("自动回复", caretaker.auto_reply || "未设定"));
  if (caretaker.patrol?.outcome) {
    caretakerSection.appendChild(createDetailRow("巡视结果", caretaker.patrol.outcome));
  }
  if (Array.isArray(caretaker.messages) && caretaker.messages.length) {
    const visitorList = document.createElement("div");
    visitorList.className = "chat-detail-list";
    for (const message of caretaker.messages.slice(0, 3)) {
      const item = document.createElement("div");
      item.className = "caretaker-note";
      item.appendChild(
        createLine(
          "caretaker-note-title",
          `${message.visitor} · ${message.urgency || "普通"}`,
        ),
      );
      item.appendChild(createLine("caretaker-note-copy", message.note));
      visitorList.appendChild(item);
    }
    caretakerSection.appendChild(visitorList);
  }
  if (Array.isArray(caretaker.notifications) && caretaker.notifications.length) {
    const notificationList = document.createElement("div");
    notificationList.className = "chat-detail-list";
    for (const note of caretaker.notifications.slice(0, 2)) {
      const item = document.createElement("div");
      item.className = "caretaker-note caretaker-note-alert";
      item.appendChild(createLine("caretaker-note-title", "给主人的提醒"));
      item.appendChild(createLine("caretaker-note-copy", note));
      notificationList.appendChild(item);
    }
    caretakerSection.appendChild(notificationList);
  }
  return caretakerSection;
}

function createChatDetailActionsSection(room, shellPage) {
  const actions = createDetailSection("快捷动作");
  const actionRow = document.createElement("div");
  actionRow.className = "chat-detail-actions";

  const refreshButton = document.createElement("button");
  refreshButton.type = "button";
  refreshButton.className = "secondary";
  refreshButton.textContent = "刷新";
  refreshButton.disabled = !gatewayUrl;
  refreshButton.addEventListener("click", async () => {
    if (!gatewayUrl) return;
    await refreshFromGateway();
  });
  actionRow.appendChild(refreshButton);

  const exportButton = document.createElement("button");
  exportButton.type = "button";
  exportButton.className = "secondary";
  exportButton.textContent = "导出当前";
  exportButton.disabled = !gatewayUrl || !activeRoomId;
  exportButton.addEventListener("click", () => {
    void exportCurrentConversation("导出当前会话失败");
  });
  actionRow.appendChild(exportButton);

  if (shellPage !== "user") {
    const worldButton = document.createElement("button");
    worldButton.type = "button";
    worldButton.className = "secondary";
    worldButton.textContent = roomKind(room) === "direct" ? "去找人" : "去找房间";
    worldButton.addEventListener("click", () => {
      setWorkspace("world");
    });
    actionRow.appendChild(worldButton);
  }

  actions.appendChild(actionRow);
  return actions;
}

function renderChatDetailPanel() {
  const shellPage = currentShellPage();
  if (currentShellPage() === "user") {
    ensureUserSceneChrome();
  }
  if (!chatDetailContentEl) return;
  clearChildren(chatDetailContentEl);

  const room = state.rooms.find((item) => item.id === activeRoomId);
  if (!room) {
    const empty = createDetailSection(
      "当前房间卡片",
      gatewayUrl
        ? "先从左侧打开一个会话，底部会显示房间卡片、状态和快捷动作。"
        : "连接网关后，这里会展示房间卡片、状态和快捷操作。",
    );
    chatDetailContentEl.appendChild(empty);
    return;
  }

  const hero = createChatDetailHeroNode(room, shellPage);
  chatDetailContentEl.appendChild(hero);

  const runtime = createChatRuntimeDetailSection(room, shellPage);
  chatDetailContentEl.appendChild(runtime);

  const caretakerSection = createCaretakerDetailSection(room);
  if (caretakerSection) {
    chatDetailContentEl.appendChild(caretakerSection);
  }

  appendChatDetailRoomContextSections(chatDetailContentEl, room);

  const actions = createChatDetailActionsSection(room, shellPage);
  chatDetailContentEl.appendChild(actions);
}

function createTimelineEmptyStateNode(cardSpec) {
  const empty = document.createElement("div");
  empty.className = cardSpec.className;
  const emptyTitle = document.createElement("div");
  emptyTitle.className = cardSpec.titleClassName;
  emptyTitle.textContent = cardSpec.titleText;
  const emptyCopy = document.createElement("div");
  emptyCopy.className = cardSpec.copyClassName;
  emptyCopy.textContent = cardSpec.copyText;
  const emptyAction = document.createElement("div");
  emptyAction.className = cardSpec.actionClassName;
  emptyAction.textContent = cardSpec.actionText;
  empty.appendChild(emptyTitle);
  empty.appendChild(emptyCopy);
  empty.appendChild(emptyAction);
  return empty;
}

function createTimelineTypingIndicatorNode(typingSpec) {
  const typingEl = document.createElement("div");
  typingEl.className = typingSpec.className;
  const dotsEl = document.createElement("span");
  dotsEl.className = typingSpec.dotsClassName;
  for (let i = 0; i < typingSpec.dotCount; i++) {
    const dot = document.createElement("span");
    dot.className = typingSpec.dotClassName;
    dotsEl.appendChild(dot);
  }
  typingEl.appendChild(dotsEl);
  const label = document.createElement("span");
  label.textContent = typingSpec.labelText;
  typingEl.appendChild(label);
  return typingEl;
}

function createTimelinePendingMessageRowFrameNode(rowSpec) {
  const row = document.createElement("div");
  row.className = rowSpec.rowClassName;
  Object.assign(row.dataset, rowSpec.rowDataset);
  return row;
}

function createTimelinePendingMessageAvatarNode(rowSpec) {
  const avatar = document.createElement("div");
  avatar.className = rowSpec.avatarClassName;
  avatar.textContent = rowSpec.avatarText;
  applyAvatarStyle(avatar, currentIdentity());
  return avatar;
}

function createTimelinePendingMessageMetaNode(rowSpec, message, quickContext) {
  const meta = document.createElement("div");
  meta.className = "message-meta";
  const sender = document.createElement("span");
  sender.className = "message-sender";
  sender.textContent = rowSpec.senderText;
  meta.appendChild(sender);

  const role = document.createElement("span");
  role.className = "message-role";
  role.textContent = rowSpec.roleText;
  meta.appendChild(role);
  appendTimelineMessageQuickChips(meta, message, quickContext);
  return meta;
}

function createTimelinePendingMessageHeaderNode(rowSpec, message, quickContext) {
  const header = document.createElement("div");
  header.className = "message-header";
  const meta = createTimelinePendingMessageMetaNode(rowSpec, message, quickContext);
  const timestamp = document.createElement("span");
  timestamp.className = "message-time";
  timestamp.textContent = rowSpec.timestampText;
  header.appendChild(meta);
  header.appendChild(timestamp);
  return header;
}

function createTimelinePendingRetryActionsNode(room, message, rowSpec) {
  if (!rowSpec.showRetry) return null;
  const pendingActions = document.createElement("div");
  pendingActions.className = "message-pending-actions";
  const retryButton = document.createElement("button");
  retryButton.type = "button";
  retryButton.className = "message-pending-retry";
  retryButton.dataset.pendingAction = "retry";
  retryButton.textContent = "重发";
  retryButton.addEventListener("click", () => {
    retryButton.disabled = true;
    void retryPendingEcho(room.id, message.id);
  });
  pendingActions.appendChild(retryButton);
  return pendingActions;
}

function createTimelinePendingMessageArticleNode(room, message, rowSpec, quickContext) {
  const article = document.createElement("article");
  article.className = rowSpec.articleClassName;
  Object.assign(article.dataset, rowSpec.articleDataset);
  article.appendChild(createTimelinePendingMessageHeaderNode(rowSpec, message, quickContext));
  article.appendChild(createMessageBodyNode(message, {
    quickState: quickContext.quickState,
    attachmentBase: gatewayUrl,
  }));
  const pendingActions = createTimelinePendingRetryActionsNode(room, message, rowSpec);
  if (pendingActions) {
    article.appendChild(pendingActions);
  }
  return article;
}

function createTimelinePendingMessageRowNode(room, message) {
  const rowSpec = timelinePendingMessageRowSpec({
    message,
    currentIdentity: currentIdentity(),
    badgeToken,
  });
  const quickContext = createTimelineMessageQuickContext(room, message);
  const row = createTimelinePendingMessageRowFrameNode(rowSpec);
  const avatar = createTimelinePendingMessageAvatarNode(rowSpec);
  const article = createTimelinePendingMessageArticleNode(room, message, rowSpec, quickContext);
  const stack = createTimelineMessageStackNode(article);
  row.appendChild(avatar);
  row.appendChild(stack);
  return row;
}

function createTimelineMessageRowFrameNode(rowSpec) {
  const row = document.createElement("div");
  row.className = rowSpec.className;
  Object.assign(row.dataset, rowSpec.dataset);
  if (rowSpec.grouped) {
    row.setAttribute("data-grouped", "true");
  }
  if (rowSpec.style) {
    row.setAttribute("style", rowSpec.style);
  }
  return row;
}

function createTimelineMessageAvatarNode(message, room, rowSpec) {
  const { isSelf, messageKind } = rowSpec;
  const avatar = document.createElement("div");
  avatar.className = `message-avatar message-avatar-${messageAvatarTone(message, room, isSelf)}`;
  avatar.textContent = badgeToken(
    isSelf ? currentIdentity() : message.sender,
    messageKind === "system" ? "系" : messageKind === "caretaker" ? "管" : isSelf ? "我" : "聊",
  );
  applyAvatarStyle(avatar, message.sender);
  return avatar;
}

function createTimelineMessageQuickContext(room, message) {
  const latestMessage = latestRoomMessageLike(room);
  const hasQuickAction = typeof message?.quick_action === "string" && message.quick_action.trim();
  const isLatestQuickAction = latestMessage === message && Boolean(hasQuickAction);
  return {
    isLatestQuickAction,
    quickState: isLatestQuickAction ? roomQuickState(room.id, message.quick_action) : "",
  };
}

function appendTimelineMessageQuickChips(meta, message, quickContext) {
  const actionChip = createMessageQuickActionChip(message.quick_action);
  if (actionChip) {
    meta.appendChild(actionChip);
  }
  const stateChip = createMessageQuickStateChip(message.quick_action, quickContext.quickState);
  if (stateChip) {
    meta.appendChild(stateChip);
  }
}

function createTimelineMessageMetaNode(message, room, rowSpec, quickContext) {
  const { isSelf } = rowSpec;
  const meta = document.createElement("div");
  meta.className = "message-meta";
  const sender = document.createElement("span");
  sender.className = "message-sender";
  sender.textContent = message.sender;
  meta.appendChild(sender);
  const role = document.createElement("span");
  role.className = `message-role${isSelf ? " message-role-self" : ""}`;
  role.textContent = messageRoleLabel(message, room, isSelf);
  meta.appendChild(role);
  appendTimelineMessageQuickChips(meta, message, quickContext);
  if (message?.is_edited && !message?.is_recalled) {
    const edited = document.createElement("span");
    edited.className = "message-edited";
    edited.textContent = "已编辑";
    meta.appendChild(edited);
  }
  return meta;
}

function createTimelineMessageTimestampNode(message) {
  const timestamp = document.createElement("span");
  timestamp.className = "message-time";
  timestamp.textContent = message.timestamp;
  if (message.timestamp_ms) {
    timestamp.setAttribute("data-full-time", formatDateTime(message.timestamp_ms));
  } else {
    timestamp.setAttribute("data-full-time", message.timestamp);
  }
  return timestamp;
}

function createTimelineMessageHeaderNode(message, room, rowSpec, quickContext) {
  const header = document.createElement("div");
  header.className = "message-header";
  const meta = createTimelineMessageMetaNode(message, room, rowSpec, quickContext);
  const timestamp = createTimelineMessageTimestampNode(message);
  header.appendChild(meta);
  header.appendChild(timestamp);
  return header;
}

function createTimelineReplyPreviewNode(message, messages) {
  const replyPreview = buildReplyPreview(message, messages);
  if (!replyPreview) return null;
  const replyEl = document.createElement("div");
  replyEl.className = "message-reply-preview";
  replyEl.textContent = `↩ ${replyPreview.sender}: ${replyPreview.text}`;
  return replyEl;
}

function createTimelineMessageArticleNode(message, room, messages, rowSpec, quickContext) {
  const { isSelf, messageKind } = rowSpec;
  const article = document.createElement("article");
  article.className = `message${isSelf ? " self" : ""}`;
  article.dataset.messageKind = messageKind;
  article.dataset.messageStableId = messageStableId(message) || "";
  article.appendChild(createTimelineMessageHeaderNode(message, room, rowSpec, quickContext));
  const replyEl = createTimelineReplyPreviewNode(message, messages);
  if (replyEl) {
    article.appendChild(replyEl);
  }
  const body = createMessageBodyNode(message, {
    quickState: quickContext.quickState,
    attachmentBase: gatewayUrl,
  });
  article.appendChild(body);
  return article;
}

function createTimelineMessageStackNode(article) {
  const stack = document.createElement("div");
  stack.className = "message-stack";
  stack.appendChild(article);
  return stack;
}

function createTimelineMessageRowNode({
  message,
  prevMessage,
  room,
  index,
  unreadStartIndex,
  messages,
  allowMessageGrouping,
  staggerBase,
  staggerCap,
}) {
  const rowSpec = timelineMessageRowSpec({
    message,
    prevMessage,
    room,
    currentIdentity: currentIdentity(),
    index,
    unreadStartIndex,
    messagesLength: messages.length,
    allowMessageGrouping,
    staggerBase,
    staggerCap,
  });
  const quickContext = createTimelineMessageQuickContext(room, message);
  const row = createTimelineMessageRowFrameNode(rowSpec);
  const avatar = createTimelineMessageAvatarNode(message, room, rowSpec);
  const article = createTimelineMessageArticleNode(message, room, messages, rowSpec, quickContext);
  const stack = createTimelineMessageStackNode(article);
  row.appendChild(avatar);
  row.appendChild(stack);
  return row;
}

function createTimelineDividerNode(dividerSpec) {
  const divider = document.createElement("div");
  divider.className = dividerSpec.className;
  divider.textContent = dividerSpec.text;
  return divider;
}

function renderTimelineNoRoomState(shellPage) {
  const emptyStateSpec = timelineNoRoomEmptyStateSpec({ gatewayUrl, shellPage });
  renderConversationMetaChips(null, emptyStateSpec.metaChips);
  renderThreadStatusRail(null);
  const empty = createTimelineEmptyStateNode(emptyStateSpec.card);
  timelineEl.appendChild(empty);
}

/**
 * 未授权私宅的 stage 反馈：状态条之外，在 timeline 区挂一张居中卡片说明
 * 为什么进不去、下一步做什么。下一次 focusRoom/refresh 会经
 * prepareTimelineSurface → clearChildren(timelineEl) 正常覆盖掉它。
 */
function renderPrivateRoomLockedCard(accessPrompt, displayName) {
  if (!timelineEl) return;
  const cardModel = privateRoomLockedCardModel(accessPrompt, { displayName });
  if (!cardModel) return;
  clearChildren(timelineEl);
  timelineEl.appendChild(createTimelineEmptyStateNode(cardModel));
}

function appendTimelineCommittedMessageRows(room, messages, flowSpec) {
  for (const item of timelineCommittedMessageRenderItems({
    messages,
    flowSpec,
  })) {
    if (item.type === "divider") {
      timelineEl.appendChild(createTimelineDividerNode(item.divider));
      continue;
    }
    const row = createTimelineMessageRowNode({
      room,
      ...item.rowInput,
    });
    timelineEl.appendChild(row);
  }
}

function appendTimelinePendingMessageRows(room, pending) {
  for (const message of pending) {
    const pendingRow = createTimelinePendingMessageRowNode(room, message);
    timelineEl.appendChild(pendingRow);
  }
}

function appendTimelineMessageFlowRows(room, flowSpec) {
  appendTimelineCommittedMessageRows(room, flowSpec.messages, flowSpec);
  appendTimelinePendingMessageRows(room, flowSpec.pending);
}

function timelineWasNearBottom() {
  return timelineEl && timelineEl.scrollHeight - timelineEl.scrollTop - timelineEl.clientHeight < 80;
}

function prepareTimelineSurface(room) {
  renderSceneHotspotsForRoom(room);
  clearChildren(timelineEl);
  syncRoomStageCanvas(room);
  renderConversationOverview();
  renderChatDetailPanel();
  renderThreadStatusRail(room);
}

function timelineMetaChipsForRoom(room, shellPage, unread, pendingCount) {
  return timelineMetaChips({
    room,
    activeRoomId,
    shellPage,
    bootstrapDisplayName: bootstrap.host.client_profile.display_name,
    routePrefix: bootstrap.shell.route_prefix,
    currentIdentity: currentIdentity(),
    roomKind: roomKind(room),
    roomKindLabel: translateRoomKindForShellPage(roomKind(room), shellPage),
    roomLastActivity: roomLastActivity(room),
    unread,
    hasDraft: roomHasDraft(room.id),
    pendingCount,
    sendError: Boolean(roomSendErrors[room.id]),
    isSendingMessage: messageSendInFlight(),
    lastRefreshErrorMessage: gatewaySyncController.lastErrorMessage(),
    roomChatStatusSummary: roomChatStatusSummary(room),
    roomSyncLabel: roomSyncLabel(),
    refreshInProgress: gatewaySyncController.isRefreshing(),
    providerConnectionState: provider.connection_state,
    translateProviderConnectionState,
    translateClientDisplayName,
    translateRoutePrefix,
  });
}

function renderTimelineSkeletonIfNeeded(room, localPreviewMessages, shellPage) {
  if (shouldRenderTimelineSkeletonRowsForContext({
    room,
    localPreviewMessages,
    shellPage,
    shellVariant: document.body?.dataset?.shellVariant || "",
  })) {
    renderTimelineSkeletonRows(4);
  }
}

function timelineFlowSpecForRoom(room, localPreviewMessages, shellPage, unread) {
  return timelineMessageFlowSpec({
    roomMessages: room.messages || [],
    localPreviewMessages,
    pendingMessages: visiblePendingEchoesForRoom(room),
    unread,
    shellPage,
  });
}

function appendTimelineTypingIndicator(flowSpec) {
  const typingSpec = timelineTypingIndicatorSpec(flowSpec.pending);
  if (!typingSpec) return;
  const typingEl = createTimelineTypingIndicatorNode(typingSpec);
  timelineEl.appendChild(typingEl);
}

function finishTimelineRender(room, flowSpec, wasNearBottom) {
  if (flowSpec.messages.length > 0 || flowSpec.pending.length > 0) {
    ensureScrollToBottomFab();
  }
  appendTimelineTypingIndicator(flowSpec);
  markRoomRead(room.id);
  if (room.id === activeRoomId && (followTimelineToLatest || wasNearBottom || messageSendInFlight())) {
    requestAnimationFrame(() => {
      if (timelineEl) {
        timelineEl.scrollTop = timelineEl.scrollHeight;
      }
    });
  }
  followTimelineToLatest = false;
}

function renderTimeline() {
  if (!timelineEl) return;
  const room = state.rooms.find((item) => item.id === activeRoomId);
  const shellPage = currentShellPage();
  const wasNearBottom = timelineWasNearBottom();
  prepareTimelineSurface(room);

  if (!room) {
    renderTimelineNoRoomState(shellPage);
    return;
  }

  const unread = unreadCount(room);
  const pendingCount = visiblePendingEchoCount(room);
  const metaChips = timelineMetaChipsForRoom(room, shellPage, unread, pendingCount);
  renderConversationMetaChips(room, metaChips);

  const localPreviewMessages = localPreviewMessagesForEmptyRoom(room);
  renderTimelineSkeletonIfNeeded(room, localPreviewMessages, shellPage);
  const flowSpec = timelineFlowSpecForRoom(room, localPreviewMessages, shellPage, unread);
  lastTimelineContext = { room, messages: flowSpec.messages || [] };
  appendTimelineMessageFlowRows(room, flowSpec);
  finishTimelineRender(room, flowSpec, wasNearBottom);
}

function renderGovernanceOfflineState() {
  governanceCitySurfaceRenderer.renderOffline({ gatewayUrl, shellMode });
}

function governancePendingMembersForCity(city) {
  return governance.memberships.filter(
    (item) => item.city_id === city.city_id && item.state === "PendingApproval",
  );
}

function governanceActiveMembersForCity(city) {
  return governance.memberships.filter(
    (item) =>
      item.city_id === city.city_id &&
      item.state === "Active" &&
      item.resident_id !== currentIdentity(),
  );
}

function hasGovernanceRenderTargets() {
  return !(
    !cityListEl &&
    !worldDirectoryListEl &&
    !worldMirrorSourceListEl &&
    !worldSquareListEl &&
    !worldSafetyListEl
  );
}

function renderGovernance() {
  if (!hasGovernanceRenderTargets()) return;

  if (!governance.world) {
    renderGovernanceOfflineState();
    return;
  }

  renderWorldDirectory();
  renderMirrorSources();
  renderWorldSquare();
  renderWorldSafety();
  governanceCitySurfaceRenderer.renderCities({
    world: governance.world,
    directory: governance.world_directory,
    cityCount: governance.cities.length,
    worldSquareCount: (governance.world_square || []).length,
    shellMode,
    cities: governance.cities.map((cityState) => {
      const city = cityState.profile;
      return {
        city,
        membership: membershipForCity(city.city_id),
        rooms: publicRoomsForCity(city.city_id),
        pendingMembers: governancePendingMembersForCity(city),
        activeMembers: governanceActiveMembersForCity(city),
      };
    }),
  });
}

function renderWorldDirectory() {
  worldSurfaceRenderers.renderWorldDirectory();
}

function renderMirrorSources() {
  worldSurfaceRenderers.renderMirrorSources();
}

function renderWorldSquare() {
  worldSurfaceRenderers.renderWorldSquare();
}

function renderWorldSafety() {
  worldSurfaceRenderers.renderWorldSafety();
}

function renderResidents() {
  residentSurfaceRenderer.renderResidents();
}
function renderResidentList() {
  residentSurfaceRenderer.renderResidentList();
}

function bootTransportStatus() {
  setNodeText(transportStateEl, `消息通道：${
    gatewayUrl
      ? "网关轮询中"
      : bootstrap.shell.stream_incremental_updates
        ? "支持流式更新"
        : "仅轮询模式"
  }`);
  bootScrollToBottomFab();
}

let scrollToBottomFabEl = null;

function ensureScrollToBottomFab() {
  if (!timelineEl) return;
  if (!scrollToBottomFabEl) {
    scrollToBottomFabEl = document.createElement("button");
    scrollToBottomFabEl.className = "scroll-to-bottom";
    scrollToBottomFabEl.textContent = "↓ 回到最新";
    scrollToBottomFabEl.type = "button";
    scrollToBottomFabEl.addEventListener("click", () => {
      if (timelineEl) {
        timelineEl.scrollTo({ top: timelineEl.scrollHeight, behavior: "smooth" });
      }
    });
  }
  if (typeof timelineEl.contains === "function" && !timelineEl.contains(scrollToBottomFabEl)) {
    timelineEl.appendChild(scrollToBottomFabEl);
  }
  updateScrollToBottomVisibility();
}

function bootScrollToBottomFab() {
  if (!timelineEl) return;
  timelineEl.addEventListener("scroll", updateScrollToBottomVisibility, { passive: true });
}

function updateScrollToBottomVisibility() {
  if (!timelineEl || !scrollToBottomFabEl) return;
  const nearBottom =
    timelineEl.scrollHeight - timelineEl.scrollTop - timelineEl.clientHeight < 120;
  scrollToBottomFabEl.dataset.visible = String(!nearBottom);
}

function ensureChatPaneToggle() {
  if (!conversationPanelEl) return;
  let toggleEl = conversationPanelEl.querySelector(".chat-pane-toggle");
  if (!toggleEl) {
    toggleEl = document.createElement("button");
    toggleEl.className = "chat-pane-toggle";
    toggleEl.type = "button";
    toggleEl.addEventListener("click", () => {
      const current = document.body.getAttribute("data-chat-pane-mode") || "thread";
      const isUserShell = document.body.getAttribute("data-shell-page") === "user";
      let next;
      if (isUserShell) {
        next = current === "thread" ? "rooms" : current === "rooms" ? "detail" : "thread";
      } else {
        next = current === "thread" ? "rooms" : "thread";
      }
      document.body.setAttribute("data-chat-pane-mode", next);
      syncChatPaneMode(next === "thread" ? "thread" : next === "detail" ? "detail" : "rooms");
      updateChatPaneToggleLabel(toggleEl, next);
    });
    conversationPanelEl.insertBefore(toggleEl, conversationPanelEl.firstChild);
  }
  const mode = document.body.getAttribute("data-chat-pane-mode") || "thread";
  updateChatPaneToggleLabel(toggleEl, mode);
}

function updateChatPaneToggleLabel(el, mode) {
  if (mode === "detail") {
    el.textContent = "← 返回消息";
  } else {
    el.textContent = mode === "thread" ? "← 查看会话列表" : "← 返回消息";
  }
}

function refreshGatewayBadge() {
  const gatewayStatus = gatewayConnectionStatus();
  document.body.dataset.gatewayConnection = gatewayStatus;
  const hudStatusEl = document.querySelector("#hud-status");
  if (hudStatusEl) {
    hudStatusEl.dataset.connection = gatewayStatus;
    hudStatusEl.textContent =
      gatewayStatus === "online" ? "在线" : gatewayStatus === "connecting" ? "连线中" : "离线";
  }
  if (gatewayUrl) {
    try {
      setNodeText(gatewayStateEl, `连接入口：${new URL(gatewayUrl).host}`);
    } catch {
      setNodeText(gatewayStateEl, `连接入口：${gatewayUrl}`);
    }
  } else {
    setNodeText(gatewayStateEl, "连接入口：未连接");
  }
  if (!gatewayUrl) {
    setNodeText(providerStateEl, "消息来源：未连接");
    return;
  }
  const mode = provider.mode || "unknown";
  const health = translateProviderHealth(provider.reachable);
  const upstreamHost = provider.base_url
    ? (() => {
        try {
          return new URL(provider.base_url).host;
        } catch {
          return provider.base_url;
        }
      })()
    : "local";
  const connectionState = translateProviderConnectionState(provider.connection_state);
  setNodeText(
    providerStateEl,
    `消息来源：${translateProviderMode(mode)} · ${health} · ${connectionState} · ${
      upstreamHost === "local" ? "本地" : upstreamHost
    }`,
  );
  if (provider.base_url && providerUrlInputEl && document.activeElement !== providerUrlInputEl) {
    providerUrlInputEl.value = provider.base_url;
  }
}

function updateComposerStatus() {
  if (!composerStatusEl) return;
  const shellPage = currentShellPage();
  const status = composerStatusState({
    shellPage,
    gatewayUrl,
    activeRoomId,
    roomSendErrors,
    lastRefreshErrorMessage: gatewaySyncController.lastErrorMessage(),
    isSendingMessage: messageSendInFlight(),
    draftText: activeRoomId ? draftForRoom(activeRoomId) : "",
    quickAction: activeRoomId ? roomQuickAction(activeRoomId) : "",
    syncLabel: roomSyncLabel(),
    gatewayUnavailable: gatewayUnavailableForComposer(),
    loginRequired: residentGatewayLoginRequired(),
    quickActionDraftStatusCopyFn: quickActionDraftStatusCopy,
  });
  composerStatusEl.textContent = status.text;
  composerStatusEl.classList.remove(
    "composer-status-muted",
    "composer-status-accent",
    "composer-status-warning",
    "composer-status-danger",
  );
  composerStatusEl.classList.add(`composer-status-${status.tone}`);
  updateCaretakerStatus();
}

function composerStateModel(room) {
  const shellPage = currentShellPage();
  const compactChatShell = shellPage === "user" || shellPage === "admin";
  const draftText = composerInputEl?.value.trim() || "";
  const syntheticIdentity = allowsSyntheticGatewayIdentity();
  const composerAvailability = computeComposerAvailability({
    hasActiveRoom: Boolean(activeRoomId),
    hasDraftText: Boolean(draftText),
    isSendingMessage: messageSendInFlight(),
    hasGateway: Boolean(gatewayUrl),
    hasGatewaySession: !gatewayUrl || Boolean(getSessionToken()) || syntheticIdentity,
    hasIdentity: userShellProjection() ? !isVisitorIdentity(currentIdentity()) : Boolean(currentIdentity()),
    requiresIdentity: userShellProjection(),
    allowSyntheticIdentity: syntheticIdentity,
    gatewayUnavailable: gatewayUnavailableForComposer(),
  });
  return { shellPage, compactChatShell, composerAvailability };
}

function applyComposerFormState(room, shellPage, composerAvailability) {
  if (composerFormEl) {
    composerFormEl.dataset.shellMode = shellMode;
    composerFormEl.dataset.draftState = composerAvailability.draftState;
    setDatasetFlag(composerFormEl, "quickAction", room ? roomQuickAction(room.id) : "");
  }
}

function composerPlaceholderForState(room, shellPage, compactChatShell, composerAvailability) {
  return resolveComposerPlaceholderForState({
    room,
    shellPage,
    compactChatShell,
    composerAvailability,
    isSendingMessage: messageSendInFlight(),
    gatewayUnavailable: gatewayUnavailableForComposer(),
    loginRequired: residentGatewayLoginRequired(),
    gatewayUrl,
    editingMessage: Boolean(editingMessageTarget),
    roomKind: room ? roomKind(room) : "",
    roomThreadHeadline: room ? roomThreadHeadline(room) : "",
    roomDisplayPeer: room ? roomDisplayPeer(room) : "",
  });
}

function applyComposerInputState(room, shellPage, compactChatShell, composerAvailability) {
  const isSendingMessage = messageSendInFlight();
  composerInputEl.disabled = !composerAvailability.canDraft || isSendingMessage;
  composerSendEl.disabled = !composerAvailability.canSend;
  const placeholder = composerPlaceholderForState(room, shellPage, compactChatShell, composerAvailability);
  composerInputEl.placeholder = placeholder;
  composerInputEl.enterKeyHint = "send";
  composerInputEl.setAttribute("aria-label", placeholder);
  composerSendEl.textContent = isSendingMessage
    ? "发送中..."
    : editingMessageTarget
      ? "保存"
      : quickActionSendLabel(room ? roomQuickAction(room.id) : "");
  composerFormEl.classList.toggle("is-sending", isSendingMessage);
  composerFormEl.classList.toggle("is-editing-message", Boolean(editingMessageTarget));
  composerFormEl.dataset.composerPage = shellPage;
}

function renderComposerDependentSurfaces(room) {
  syncUserQuickActionButtons(room?.id || activeRoomId);
  updateComposerStatus();
  renderComposerHero(room);
  updateComposerContext(room);
  updateComposerTip();
  renderComposerMeta(room);
}

function updateComposerState() {
  ensureComposerTip();
  ensureComposerKeyBindings();
  const room = state.rooms.find((item) => item.id === activeRoomId);
  const { shellPage, compactChatShell, composerAvailability } = composerStateModel(room);
  applyComposerFormState(room, shellPage, composerAvailability);
  if (!composerFormEl || !composerInputEl || !composerSendEl) {
    updateComposerStatus();
    return;
  }
  applyComposerInputState(room, shellPage, compactChatShell, composerAvailability);
  renderComposerDependentSurfaces(room);
}

async function submitComposerMessage() {
  if (composerSubmitBlocked()) return false;
  const draft = composerSubmitDraft();
  if (!draft) return false;
  if (editingMessageTarget) return submitComposerEditTarget(draft.text);
  return submitComposerNewMessage(draft.text, draft.quickAction);
}

function composerSubmitBlocked() {
  if (messageSendInFlight()) {
    updateComposerState();
    return true;
  }
  if (residentGatewayLoginRequired()) {
    setAuthStatus("请先登录后发送", true);
    updateResidentLoginSurface();
    updateComposerState();
    return true;
  }
  if (!activeRoomId) {
    updateComposerState();
    return true;
  }
  return false;
}

function composerSubmitDraft() {
  const text = composerInputEl.value.trim();
  const quickAction = roomQuickAction(activeRoomId);
  if (!text) return null;
  composerSendEl.disabled = true;
  return { text, quickAction };
}

function renderComposerSubmitSurfaces() {
  renderRooms();
  renderTimeline();
  renderConversationOverview();
}

function renderComposerSubmitFailure(roomId, message) {
  roomSendErrors[roomId] = message;
  refreshGatewayBadge();
  renderComposerSubmitSurfaces();
}

async function submitComposerEditTarget(text) {
  const target = editingMessageTarget;
  if (target.roomId !== activeRoomId) {
    clearMessageEditTarget({ clearInput: true });
    updateComposerState();
    return false;
  }
  try {
    await editMessage(target.roomId, target.messageId, text);
    clearMessageEditTarget({ clearInput: true });
    updateRoomDraft(target.roomId, "");
    delete roomSendErrors[target.roomId];
    renderComposerSubmitSurfaces();
  } catch (error) {
    renderComposerSubmitFailure(target.roomId, localizedRuntimeError(error, "消息编辑失败"));
    return false;
  } finally {
    updateComposerState();
  }
  return true;
}

async function submitComposerNewMessage(text, quickAction) {
  try {
    await sendMessage(text, { quickAction });
  } catch (error) {
    const message = localizedRuntimeError(error, "消息发送失败");
    renderComposerSubmitFailure(activeRoomId, message);
    return false;
  } finally {
    updateComposerState();
  }
  return true;
}

function governanceWorldStewardInputElements() {
  return new Set([
    worldMirrorUrlInputEl,
    worldNoticeTitleInputEl,
    worldNoticeSeveritySelectEl,
    worldNoticeTagsInputEl,
    worldNoticeBodyInputEl,
    worldTrustCityInputEl,
    worldTrustStateSelectEl,
    worldTrustReasonInputEl,
    worldAdvisorySubjectKindSelectEl,
    worldAdvisorySubjectInputEl,
    worldAdvisoryActionInputEl,
    worldAdvisoryReasonInputEl,
    worldReportReviewIdInputEl,
    worldReportReviewStatusSelectEl,
    worldReportReviewCityStateSelectEl,
    worldReportReviewResolutionInputEl,
    worldResidentIdInputEl,
    worldResidentCityInputEl,
    worldResidentEmailInputEl,
    worldResidentMobileInputEl,
    worldResidentReasonInputEl,
  ]);
}

function governanceManagedInputElements() {
  return [
    providerUrlInputEl,
    cityTitleInputEl,
    citySlugInputEl,
    cityDescriptionInputEl,
    cityJoinInputEl,
    roomCityInputEl,
    roomTitleInputEl,
    roomSlugInputEl,
    roomDescriptionInputEl,
    directPeerInputEl,
    worldMirrorUrlInputEl,
    worldNoticeTitleInputEl,
    worldNoticeSeveritySelectEl,
    worldNoticeTagsInputEl,
    worldNoticeBodyInputEl,
    worldTrustCityInputEl,
    worldTrustStateSelectEl,
    worldTrustReasonInputEl,
    worldAdvisorySubjectKindSelectEl,
    worldAdvisorySubjectInputEl,
    worldAdvisoryActionInputEl,
    worldAdvisoryReasonInputEl,
    worldReportReviewIdInputEl,
    worldReportReviewStatusSelectEl,
    worldReportReviewCityStateSelectEl,
    worldReportReviewResolutionInputEl,
    worldReportCityInputEl,
    worldReportTargetKindSelectEl,
    worldReportTargetInputEl,
    worldReportSummaryInputEl,
    worldReportEvidenceInputEl,
    worldResidentIdInputEl,
    worldResidentCityInputEl,
    worldResidentEmailInputEl,
    worldResidentMobileInputEl,
    worldResidentReasonInputEl,
  ];
}

function updateGovernanceManagedInputs(enabled, worldStewardEnabled) {
  const worldStewardInputs = governanceWorldStewardInputElements();
  for (const element of governanceManagedInputElements()) {
    if (!element) continue;
    element.disabled = worldStewardInputs.has(element) ? !worldStewardEnabled : !enabled;
  }
}

function governanceButtonStateDescriptors(enabled, worldStewardEnabled) {
  return [
    { element: cityCreateFormEl?.querySelector("button"), disabled: !enabled },
    { element: cityJoinFormEl?.querySelector("button"), disabled: !enabled },
    { element: roomCreateFormEl?.querySelector("button"), disabled: !enabled },
    { element: directOpenFormEl?.querySelector("button"), disabled: !enabled },
    { element: providerConnectFormEl?.querySelector("button"), disabled: !enabled },
    { element: providerDisconnectButtonEl, disabled: !enabled || !provider.base_url },
    { element: worldMirrorFormEl?.querySelector("button"), disabled: !worldStewardEnabled },
    { element: worldNoticeFormEl?.querySelector("button"), disabled: !worldStewardEnabled },
    { element: worldTrustFormEl?.querySelector("button"), disabled: !worldStewardEnabled },
    { element: worldAdvisoryFormEl?.querySelector("button"), disabled: !worldStewardEnabled },
    { element: worldReportReviewFormEl?.querySelector("button"), disabled: !worldStewardEnabled },
    { element: worldReportFormEl?.querySelector("button"), disabled: !enabled },
    { element: worldResidentSanctionFormEl?.querySelector("button"), disabled: !worldStewardEnabled },
  ];
}

function updateGovernanceManagedButtons(enabled, worldStewardEnabled) {
  for (const { element, disabled } of governanceButtonStateDescriptors(enabled, worldStewardEnabled)) {
    if (element) element.disabled = disabled;
  }
}

function updateGovernanceFormState() {
  const enabled = Boolean(gatewayUrl && currentIdentity());
  const worldStewardEnabled = enabled && actorIsWorldSteward();
  updateGovernanceManagedInputs(enabled, worldStewardEnabled);
  updateGovernanceManagedButtons(enabled, worldStewardEnabled);
}

function updateAuthFormState() {
  return updateAuthFormStateMod();
}

function resolveGatewayUrl() {
  const query = queryGatewayUrl();
  if (query) {
    safeLocalStorageSet("lobster-gateway-url", query);
  }
  return resolveGatewayUrlCandidate({
    shellPage: currentShellPage(),
    queryGateway: query,
    rememberedGateway: safeLocalStorageGet("lobster-gateway-url"),
    bootstrapGatewayBaseUrl: bootstrap.gateway_base_url,
    protocol: window.location.protocol,
    origin: window.location.origin,
    userProjection: userShellProjection(),
  });
}

function handleGatewayAuthFailure(status) {
  const handled = handleGatewayAuthFailureMod(status);
  if (handled) applyRailVisibility();
  return handled;
}

function sceneEditorGatewayUrl() {
  return gatewayUrl || safeLocalStorageGet("lobster-gateway-url") || "";
}

async function postGatewayJson(path, payload) {
  const headers = gatewayJsonHeaders(getSessionToken());
  const response = await fetch(`${gatewayUrl}${path}`, {
    method: "POST",
    headers,
    body: JSON.stringify(payload),
  });
  const text = await response.text();
  let parsed = null;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    // ignore parse errors
  }
  if (!response.ok) {
    const message = gatewayErrorMessage(parsed, text, response.status);
    handleGatewayAuthFailure(response.status);
    throw new Error(message);
  }
  return parsed;
}

async function submitPersonalRoomAccessPolicy(policy) {
  const room = activeRoom();
  const requestState = personalRoomAccessPolicySubmitRequestState({
    policy,
    room,
    currentIdentity: currentIdentity(),
    gatewayUrl,
    roomOwnershipForState,
  });
  if (!requestState.allowed) {
    if (requestState.statusText) {
      setPersonalRoomAccessPolicyStatus(requestState.statusText, requestState.statusIsError);
    }
    if (requestState.shouldSyncControl) {
      syncPersonalRoomAccessPolicyControl();
    }
    return;
  }

  personalRoomAccessPolicySaving = true;
  syncPersonalRoomAccessPolicyControl();
  let finalStatus = "";
  let finalStatusIsError = false;
  try {
    const response = await postGatewayJson(requestState.endpoint, requestState.payload);
    room.personal_room_access_policy = appliedPersonalRoomAccessPolicy(response, policy);
    await refreshFromGateway({ requireShell: true });
    finalStatus = "已保存";
  } catch (error) {
    finalStatus = localizedRuntimeError(error, "保存失败");
    finalStatusIsError = true;
  } finally {
    personalRoomAccessPolicySaving = false;
    syncPersonalRoomAccessPolicyControl();
    if (finalStatus) {
      setPersonalRoomAccessPolicyStatus(finalStatus, finalStatusIsError);
    }
  }
}

async function refreshFromGateway({ requireShell = false } = {}) {
  return gatewaySyncController.refresh({ requireShell });
}

function registerUnhandledRuntimeReporter() {
  window.addEventListener("unhandledrejection", (event) => {
    gatewaySyncController.recordFailure(event.reason, "前端运行异常");
    renderShellStateRefresh();
  });
}

function renderShellStateRefresh() {
  renderRooms();
  renderTimeline();
  updateComposerState();
  updateAuthFormState();
  updateResidentLoginSurface();
  applyRailVisibility();
  syncPersonalRoomAccessPolicyControl();
  renderConversationOverview();
}

function startGatewayRealtime(options = {}) {
  return gatewayRealtimeController.start(options);
}

function appendLocalRoomMessage(roomId, text, quickAction) {
  const room = state.rooms.find((item) => item.id === roomId);
  if (!room) return false;
  const timestamp = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  room.messages = room.messages || [];
  room.messages.push({
    sender: currentIdentity(),
    timestamp,
    text,
    quick_action: quickAction,
  });
  return true;
}

function captureQuickActionSend(roomId, text, quickAction) {
  if (!quickAction) return;
  const initialState = resetRoomQuickState(roomId, quickAction);
  captureRoomQuickSnapshotFromText(roomId, quickAction, initialState, text);
}

function clearComposerAfterSend(roomId, text) {
  composerInputEl.value = "";
  autoSizeComposerInput();
  updateRoomDraft(roomId, "");
  setRoomQuickAction(roomId, "");
  lastSentMessage = text;
}

function renderAfterSend({ composerFirst = false } = {}) {
  if (composerFirst) updateComposerState();
  renderRooms();
  renderTimeline();
  if (!composerFirst) updateComposerState();
  renderConversationOverview();
}

function focusComposerAfterSend() {
  requestAnimationFrame(() => {
    composerInputEl?.focus();
  });
}

function commitLocalSend(roomId, text, quickAction) {
  if (!appendLocalRoomMessage(roomId, text, quickAction)) return false;
  captureQuickActionSend(roomId, text, quickAction);
  gatewaySyncController.recordSuccess();
  followTimelineToLatest = true;
  delete roomSendErrors[roomId];
  clearComposerAfterSend(roomId, text);
  renderAfterSend();
  focusComposerAfterSend();
  return true;
}

function gatewayMessagePayload(roomId, text, quickAction, attachmentId = "") {
  return gatewayMessagePayloadForState(roomId, text, quickAction, {
    currentIdentity,
    languageTag: navigator.language,
  }, attachmentId);
}
function prepareGatewaySend(roomId, text, quickAction) {
  followTimelineToLatest = true;
  const pendingEchoId = enqueuePendingEcho(roomId, text, quickAction);
  captureQuickActionSend(roomId, text, quickAction);
  clearComposerAfterSend(roomId, text);
  renderAfterSend({ composerFirst: true });
  return pendingEchoId;
}

function handleGatewaySendFailure(roomId, pendingEchoId, posted, error) {
  markPendingEchoFailed(roomId, pendingEchoId, true);
  const fallback = posted ? "消息可能已发出，但会话同步失败" : "消息发送失败";
  roomSendErrors[roomId] = localizedRuntimeError(error, fallback);
  return new Error(roomSendErrors[roomId]);
}

function finishGatewaySendAttempt() {
  updateComposerState();
  renderConversationOverview();
  focusComposerAfterSend();
}

async function sendMessage(text, { quickAction = "", attachmentId = "" } = {}) {
  return messageSendController.send(text, { quickAction, attachmentId });
}

async function uploadImageAttachment(file) {
  if (!gatewayUrl) throw new Error("请先连接网关后再发送图片");
  if (!(file instanceof File)) throw new Error("请选择一张图片");
  if (!/^image\/(png|jpe?g|gif|webp)$/i.test(file.type)) {
    throw new Error("仅支持 png / jpg / gif / webp 图片");
  }
  // 上传前压缩（大图降尺寸 + 重编码）；任何失败回退原图，
  // Gateway 的魔数嗅探与 5MB 上限仍是最终兜底。
  let payload = file;
  try {
    payload = await compressImageFile(file);
  } catch {
    payload = file;
  }
  if (payload.size > 5 * 1024 * 1024) {
    throw new Error("图片过大：单张最大 5MB");
  }
  const headers = { "Content-Type": payload.type };
  const sessionToken = getSessionToken();
  if (sessionToken) headers.Authorization = `Bearer ${sessionToken}`;
  const response = await fetch(`${gatewayUrl}/v1/shell/attachment`, {
    method: "POST",
    headers,
    body: payload,
  });
  const text = await response.text();
  let parsed = null;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    // ignore parse errors
  }
  if (!response.ok) {
    const message = gatewayErrorMessage(parsed, text, response.status);
    handleGatewayAuthFailure(response.status);
    throw new Error(message);
  }
  if (!parsed?.attachment_id) throw new Error("图片上传失败");
  return parsed;
}

async function submitComposerAttachment() {
  if (composerSubmitBlocked()) return false;
  const input = document.querySelector("#composer-attachment-input");
  const file = input?.files?.[0];
  if (!file) return false;
  const draftText = (composerInputEl?.value || "").trim();
  try {
    composerSendEl.disabled = true;
    const uploaded = await uploadImageAttachment(file);
    await sendMessage(draftText, { attachmentId: uploaded.attachment_id });
    if (composerInputEl) composerInputEl.value = "";
    renderComposerSubmitSurfaces();
    return true;
  } catch (error) {
    const message = localizedRuntimeError(error, "图片发送失败");
    renderComposerSubmitFailure(activeRoomId, message);
    return false;
  } finally {
    if (input) input.value = "";
    updateComposerState();
  }
}
async function editMessage(roomId, messageId, text) {
  if (!gatewayUrl) {
    throw new Error("请先连接网关后再编辑消息");
  }
  if (!roomId || !messageId || !text.trim()) {
    throw new Error("编辑消息需要提供房间、消息 ID 和新内容");
  }
  await postGatewayJson(
    "/v1/shell/message/edit",
    editMessagePayloadForState(roomId, messageId, text, { currentIdentity }),
  );
  await refreshFromGateway({ requireShell: true });
}

async function recallMessage(roomId, messageId) {
  if (!gatewayUrl) {
    throw new Error("请先连接网关后再撤回消息");
  }
  if (!roomId || !messageId) {
    throw new Error("撤回消息需要提供房间和消息 ID");
  }
  await postGatewayJson(
    "/v1/shell/message/recall",
    recallMessagePayloadForState(roomId, messageId, { currentIdentity }),
  );
  await refreshFromGateway({ requireShell: true });
}

async function submitCreateCity() {
  if (!gatewayUrl) return;
  const title = cityTitleInputEl.value.trim();
  const slug = citySlugInputEl.value.trim();
  const description = cityDescriptionInputEl.value.trim();
  if (!title || !description) {
    setGovernanceStatus("请填写城市名称和城市简介", true);
    return;
  }
  setGovernanceStatus(`正在创建城市：${title}`);
  await postGatewayJson("/v1/cities", {
    title,
    slug: slug || undefined,
    description,
    lord_id: currentIdentity(),
  });
  cityCreateFormEl?.reset();
  await refreshFromGateway();
  setGovernanceStatus(`城市已创建：${title}`);
}

async function submitJoinCity(cityToken = null) {
  if (!gatewayUrl) return;
  const city = (cityToken || cityJoinInputEl.value).trim();
  if (!city) {
    setGovernanceStatus("请填写城市别名或城市标识", true);
    return;
  }
  setGovernanceStatus(`正在申请加入：${city}`);
  const result = await postGatewayJson("/v1/cities/join", {
    city,
    resident_id: currentIdentity(),
  });
  cityJoinFormEl?.reset();
  await refreshFromGateway();
  setGovernanceStatus(`入城申请状态：${translateMembershipState(result.state)}`);
}

async function submitCreateRoom() {
  if (!gatewayUrl) return;
  const city = roomCityInputEl.value.trim();
  const title = roomTitleInputEl.value.trim();
  const slug = roomSlugInputEl.value.trim();
  const description = roomDescriptionInputEl.value.trim();
  if (!city || !title || !description) {
    setGovernanceStatus("请填写城市、房间名称和房间简介", true);
    return;
  }
  setGovernanceStatus(`正在创建房间：${title}`);
  const result = await postGatewayJson("/v1/cities/rooms", {
    city,
    creator_id: currentIdentity(),
    title,
    slug: slug || undefined,
    description,
  });
  roomCreateFormEl?.reset();
  focusRoom(result.room_id);
  await refreshFromGateway();
  setGovernanceStatus(`房间已创建：${result.title}`);
}

async function submitApproveResident(city, residentId) {
  setGovernanceStatus(`正在批准 ${residentId} 加入 ${city}`);
  await postGatewayJson("/v1/cities/approve", {
    city,
    actor_id: currentIdentity(),
    resident_id: residentId,
  });
  await refreshFromGateway();
  setGovernanceStatus(`${residentId} 已通过 ${city} 的入城审批`);
}

async function submitFreezeRoom(city, room, frozen) {
  setGovernanceStatus(`${frozen ? "正在冻结" : "正在解冻"}房间：${room}`);
  await postGatewayJson("/v1/cities/rooms/freeze", {
    city,
    actor_id: currentIdentity(),
    room,
    frozen,
  });
  await refreshFromGateway();
  setGovernanceStatus(`房间 ${room} 已${frozen ? "冻结" : "解冻"}`);
}

async function submitStewardUpdate(city, residentId, grant) {
  setGovernanceStatus(`${grant ? "正在授予" : "正在撤销"} ${residentId} 的执事身份`);
  await postGatewayJson("/v1/cities/stewards", {
    city,
    actor_id: currentIdentity(),
    resident_id: residentId,
    grant,
  });
  await refreshFromGateway();
  setGovernanceStatus(`${residentId} 当前身份：${grant ? "执事" : "居民"}`);
}

async function submitFederationPolicy(city, policy) {
  setGovernanceStatus(`正在更新 ${city} 的联邦策略为 ${translateFederationPolicy(policy)}`);
  await postGatewayJson("/v1/cities/federation-policy", {
    city,
    actor_id: currentIdentity(),
    policy,
  });
  await refreshFromGateway();
  setGovernanceStatus(`${city} 的联邦策略已切换为 ${translateFederationPolicy(policy)}`);
}

/**
 * Enter a resident's personal room, preferring the gateway-provided
 * personal_room_id over a fresh /v1/direct/open roundtrip.
 */
async function enterResidentRoom(resident) {
  if (!gatewayUrl) return;
  const displayName = resident.nickname || resident.resident_id;
  const accessPrompt = residentPrivateRoomAccessPromptModel(resident, {
    currentResidentId: currentIdentity(),
    roomVisible: state.rooms.some((room) => room.id === resident.personal_room_id),
  });
  if (accessPrompt) {
    setGovernanceStatus(accessPrompt.text, accessPrompt.isError, accessPrompt.className);
    renderPrivateRoomLockedCard(accessPrompt, displayName);
    return;
  }
  const message = "进入「" + displayName + "」的房间私聊？";
  if (typeof window.confirm === "function" && !window.confirm(message)) return;

  // If gateway already knows the personal room, navigate directly.
  if (resident.personal_room_id) {
    focusRoom(resident.personal_room_id);
    await refreshFromGateway();
    // 空个人房间无消息，prepareTimelineSurface 不会触发 stage 渲染；
    // 显式同步 stage canvas，确保主客徽章 + image_layer 背景生效
    const activeRoom = state.rooms.find((r) => r.id === activeRoomId);
    if (activeRoom) syncRoomStageCanvas(activeRoom);
    setGovernanceStatus("私聊已就绪：" + displayName);
    return;
  }
  // Fall back to opening a fresh direct session.
  await openDirectSession(resident.resident_id);
}

async function openDirectSession(peerId) {
  const requestState = directSessionOpenRequestState({
    peerId,
    currentIdentity: currentIdentity(),
    gatewayUrl,
  });
  if (!requestState.allowed) {
    if (requestState.statusText) {
      setGovernanceStatus(requestState.statusText, requestState.statusIsError);
    }
    return;
  }
  setGovernanceStatus(requestState.statusText);
  const result = await postGatewayJson(requestState.endpoint, requestState.payload);
  directOpenFormEl?.reset();
  focusRoom(result.conversation_id);
  await refreshFromGateway();
  setGovernanceStatus(requestState.successText);
}

async function submitProviderConnect() {
  if (!gatewayUrl) return;
  const providerUrl = providerUrlInputEl.value.trim();
  if (!providerUrl) {
    setGovernanceStatus("请填写消息来源地址", true);
    return;
  }
  setGovernanceStatus(`正在连接消息来源：${providerUrl}`);
  provider = await postGatewayJson("/v1/provider/connect", {
    provider_url: providerUrl,
  });
  await refreshFromGateway();
  setGovernanceStatus(`消息来源已连接：${translateProviderMode(provider.mode)}`);
}

async function submitProviderDisconnect() {
  if (!gatewayUrl) return;
  setGovernanceStatus("正在断开消息来源");
  provider = await postGatewayJson("/v1/provider/disconnect", {});
  if (!provider.base_url) {
    providerUrlInputEl.value = "";
  }
  await refreshFromGateway();
  setGovernanceStatus("消息来源已断开，当前改用本地草稿");
}

async function submitAddMirrorSource() {
  const baseUrl = worldMirrorUrlInputEl.value.trim();
  if (!baseUrl) {
    setGovernanceStatus("请填写镜像源地址", true);
    return;
  }
  setGovernanceStatus(`正在添加镜像源：${baseUrl}`);
  await postGatewayJson("/v1/world-mirror-sources", {
    base_url: baseUrl,
  });
  worldMirrorFormEl.reset();
  await refreshFromGateway();
  setGovernanceStatus(`镜像源已添加：${baseUrl}`);
}

async function submitWorldNotice() {
  const title = worldNoticeTitleInputEl.value.trim();
  const body = worldNoticeBodyInputEl.value.trim();
  if (!title || !body) {
    setGovernanceStatus("请填写公告标题和正文", true);
    return;
  }
  const tags = worldNoticeTagsInputEl.value
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean);
  setGovernanceStatus(`正在发布世界公告：${title}`);
  await postGatewayJson("/v1/world-square/notices", {
    actor_id: currentIdentity(),
    title,
    body,
    severity: worldNoticeSeveritySelectEl.value || "info",
    tags,
  });
  worldNoticeFormEl.reset();
  worldNoticeSeveritySelectEl.value = "info";
  await refreshFromGateway();
  setGovernanceStatus(`世界公告已发布：${title}`);
}

async function submitCityTrustUpdate() {
  const city = worldTrustCityInputEl.value.trim();
  const reason = worldTrustReasonInputEl.value.trim();
  if (!city) {
    setGovernanceStatus("请填写城市别名或城市标识", true);
    return;
  }
  setGovernanceStatus(`正在更新 ${city} 的信任状态`);
  await postGatewayJson("/v1/world-safety/cities/trust", {
    actor_id: currentIdentity(),
    city,
    state: worldTrustStateSelectEl.value,
    reason: reason || undefined,
  });
  worldTrustFormEl.reset();
  worldTrustStateSelectEl.value = "Healthy";
  await refreshFromGateway();
  setGovernanceStatus(`${city} 的信任状态已更新`);
}

async function submitWorldAdvisory() {
  const subjectRef = worldAdvisorySubjectInputEl.value.trim();
  const action = worldAdvisoryActionInputEl.value.trim();
  const reason = worldAdvisoryReasonInputEl.value.trim();
  if (!subjectRef || !action || !reason) {
    setGovernanceStatus("请填写对象、动作和原因", true);
    return;
  }
  setGovernanceStatus(`正在发布安全通告：${subjectRef}`);
  await postGatewayJson("/v1/world-safety/advisories", {
    actor_id: currentIdentity(),
    subject_kind: worldAdvisorySubjectKindSelectEl.value,
    subject_ref: subjectRef,
    action,
    reason,
  });
  worldAdvisoryFormEl.reset();
  worldAdvisorySubjectKindSelectEl.value = "City";
  await refreshFromGateway();
  setGovernanceStatus(`安全通告已发布：${subjectRef}`);
}

async function submitWorldReport() {
  const city = worldReportCityInputEl.value.trim();
  const targetRef = worldReportTargetInputEl.value.trim();
  const summary = worldReportSummaryInputEl.value.trim();
  if (!targetRef || !summary) {
    setGovernanceStatus("请填写举报对象和违规摘要", true);
    return;
  }
  const evidence = worldReportEvidenceInputEl.value
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
  setGovernanceStatus(`正在提交举报：${targetRef}`);
  await postGatewayJson("/v1/world-safety/reports", {
    reporter_id: currentIdentity(),
    city: city || undefined,
    target_kind: worldReportTargetKindSelectEl.value,
    target_ref: targetRef,
    summary,
    evidence,
  });
  worldReportFormEl.reset();
  worldReportTargetKindSelectEl.value = "City";
  await refreshFromGateway();
  setGovernanceStatus(`举报已提交：${targetRef}`);
}

async function submitWorldReportReview() {
  const reportId = worldReportReviewIdInputEl.value.trim();
  const resolution = worldReportReviewResolutionInputEl.value.trim();
  if (!reportId || !resolution) {
    setGovernanceStatus("请填写举报标识和审查结论", true);
    return;
  }
  const cityState = worldReportReviewCityStateSelectEl.value.trim();
  setGovernanceStatus(`正在审查举报：${reportId}`);
  await postGatewayJson("/v1/world-safety/reports/review", {
    actor_id: currentIdentity(),
    report_id: reportId,
    status: worldReportReviewStatusSelectEl.value,
    resolution,
    city_state: cityState || undefined,
  });
  worldReportReviewFormEl.reset();
  worldReportReviewStatusSelectEl.value = "Reviewed";
  worldReportReviewCityStateSelectEl.value = "";
  await refreshFromGateway();
  setGovernanceStatus(`举报已审查：${reportId}`);
}

async function submitResidentSanction() {
  const residentId = worldResidentIdInputEl.value.trim();
  const reason = worldResidentReasonInputEl.value.trim();
  if (!residentId || !reason) {
    setGovernanceStatus("请填写居民标识和制裁原因", true);
    return;
  }
  const city = worldResidentCityInputEl.value.trim();
  const email = worldResidentEmailInputEl.value.trim();
  const mobile = worldResidentMobileInputEl.value.trim();
  const devicePhysicalAddress = worldResidentDeviceInputEl.value.trim();
  setGovernanceStatus(`正在发布居民制裁：${residentId}`);
  await postGatewayJson("/v1/world-safety/residents/sanction", {
    actor_id: currentIdentity(),
    resident_id: residentId,
    city: city || undefined,
    email: email || undefined,
    mobile: mobile || undefined,
    device_physical_addresses: devicePhysicalAddress
      ? [devicePhysicalAddress]
      : undefined,
    reason,
    portability_revoked: true,
  });
  worldResidentSanctionFormEl.reset();
  await refreshFromGateway();
  setGovernanceStatus(`居民制裁已发布：${residentId}`);
}

async function requestEmailOtp() {
  return requestEmailOtpMod();
}

async function verifyEmailOtp() {
  return verifyEmailOtpMod();
}

async function updateMyNickname(nickname) {
  return updateMyNicknameMod(nickname);
}


async function exportCurrentConversation(fallbackMessage = "导出当前会话失败") {
  try {
    if (!activeRoomId) {
      setGovernanceStatus("请先打开一个会话", true);
      return;
    }
    await exportHistory({ conversationId: activeRoomId, includePublic: true });
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, fallbackMessage), true);
  }
}

async function exportHistory({ conversationId = null, includePublic = true } = {}) {
  if (!gatewayUrl) {
    setGovernanceStatus("请先连接网关后再导出会话", true);
    return;
  }
  const format = exportFormatSelectEl?.value || "markdown";
  const params = new URLSearchParams({
    resident_id: currentIdentity(),
    format,
    include_public: includePublic ? "true" : "false",
  });
  if (conversationId) {
    params.set("conversation_id", conversationId);
  }
  setGovernanceStatus(
    conversationId ? `正在导出会话：${conversationId}` : "正在导出全部可见历史",
  );
  const response = await fetch(`${gatewayUrl}/v1/export?${params.toString()}`, {
    headers: gatewayJsonHeaders(getSessionToken()),
  });
  let payload = null;
  try {
    payload = await response.json();
  } catch {
    payload = null;
  }
  if (!response.ok) {
    handleGatewayAuthFailure(response.status);
    throw new Error(gatewayErrorMessage(payload, "", response.status) || "导出失败");
  }
  const scopeName = conversationId ? conversationId.replace(/[:/]+/g, "_") : "全部历史";
  const filename = `我和狗蛋儿的家_${scopeName}.${exportFileExtension(format)}`;
  downloadContent(filename, payload.content || "", exportMimeType(format));
  setGovernanceStatus(`导出文件已准备好：${filename}`);
}

function initializeLocalShellState() {
  initThemeToggle();
  chatFocusController.initialize();
  applyShellMode();
  setWorkspace(resolveWorkspace(), { persist: false });
  roomReadMarkers = loadRoomReadMarkers();
  roomDrafts = loadRoomDrafts();
  roomQuickStates = loadRoomQuickStates();
  roomQuickSnapshots = loadRoomQuickSnapshots();
  syncChatPaneMode(resolveChatPaneMode(), { persist: false });
}

function updateInitialAuthStatus() {
  if (hasGatewayAuthFailureMod()) return;
  const authSession = getAuthSession();
  if (authSession.challengeId && authSession.maskedEmail) {
    setAuthStatus(`待完成验证码登录：${authSession.maskedEmail}`);
  } else {
    setAuthStatus("空闲");
  }
}

async function loadInitialRuntimeState() {
  await loadBootstrap();
  await loadCachedState();
  loadAuthDraft();
  // Keep the static/generated shell initialization independent from a
  // non-user bootstrap gateway. User pages with an explicit gateway query
  // still need the URL before identity selection so query identities cannot
  // bypass the resident session boundary.
  gatewayUrl = queryGatewayUrl() || null;
  await loadShellState();
  gatewayUrl = resolveGatewayUrl();
  loadSenderIdentity();
  updateInitialAuthStatus();
  await loadGatewayBootstrap();
  gatewayUrl = resolveGatewayUrl();
  // 网关地址/身份就绪后刷新推送开关状态（挂载时地址尚未解析，按钮先休眠）
  void pushClientInstance?.refresh();
  await refreshFromGateway();
}

function sceneEditorUrlForCurrentState() {
  const editorGatewayUrl = sceneEditorGatewayUrl();
  if (!editorGatewayUrl) return "";
  const sessionToken = safeLocalStorageGet("lobster-session-token");
  var editorUrl = "./scene-editor.html?gateway=" + encodeURIComponent(editorGatewayUrl);
  if (activeRoomId) editorUrl += "&room=" + encodeURIComponent(activeRoomId);
  if (sessionToken) editorUrl += "&token=" + encodeURIComponent(sessionToken);
  const editorIdentity = currentIdentity();
  if (sessionToken && !isVisitorIdentity(editorIdentity)) {
    editorUrl += "&identity=" + encodeURIComponent(editorIdentity);
  }
  return editorUrl;
}

function bindSceneEditorLink() {
  // scene-editor 入口 URL：init 时用当前 activeRoomId 设 href（中键/新标签可用），
  // 左键 click handler 实时刷新（owner 切换房间后跟随当前房间）。旧逻辑构造残缺
  // "dm:" + id + ":"（缺对方 id），scene-editor 用它查 /v1/shell/state 永远匹配不到
  // 房间 → owner 加载必空（P1 房间编辑器加载阻断）。
  const sceneEditorLink = document.getElementById("scene-editor-link");
  if (!sceneEditorLink) return;
  const editorUrl = sceneEditorUrlForCurrentState();
  if (editorUrl) {
    sceneEditorLink.href = editorUrl;
  }
  if (!sceneEditorLink.dataset.sceneEditorBound) {
    sceneEditorLink.dataset.sceneEditorBound = "1";
    sceneEditorLink.addEventListener("click", function (event) {
      const clickGatewayUrl = sceneEditorGatewayUrl();
      if (!clickGatewayUrl) return;
      const clickUrl = sceneEditorUrlForCurrentState();
      if (!clickUrl) return;
      event.preventDefault();
      window.location.assign(clickUrl);
    });
  }
  applyRailVisibility();
}

function renderInitialShell() {
  bootTransportStatus();
  refreshGatewayBadge();
  if (!userShellProjection()) {
    renderGovernance();
  }
  renderResidents();
  renderRooms();
  renderTimeline();
  updateComposerState();
  updateAuthFormState();
  updateResidentLoginSurface();
  syncPersonalRoomAccessPolicyControl();
  if (!userShellProjection()) {
    updateGovernanceFormState();
  }
  applyWorkspace();
  syncComposerDraft({ force: true });
  updateComposerState();
}

async function main() {
  await runShellStartup({
    initializeLocalState: initializeLocalShellState,
    loadInitialRuntimeState,
    bindSceneEditorLink,
    loadWorldEntry,
    renderInitialShell,
    startGatewayRealtime,
    focusComposerInput,
  });
}

composerFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  await submitComposerMessage();
});

document.querySelector("[data-attachment-trigger]")?.addEventListener("click", () => {
  document.querySelector("#composer-attachment-input")?.click();
});
document.querySelector("#composer-attachment-input")?.addEventListener("change", async (event) => {
  await submitComposerAttachment();
});

for (const button of personalRoomPolicyButtons) {
  button.addEventListener("click", async () => {
    await submitPersonalRoomAccessPolicy(button.dataset.personalRoomPolicy);
  });
}

bindShellForegroundLifecycle({
  refreshOnForeground: gatewayPollingController.refreshOnForeground,
});

composerInputEl?.addEventListener("input", (event) => {
  if (activeRoomId) {
    const roomId = activeRoomId;
    const hadDraft = roomHasDraft(roomId);
    const hadError = Boolean(roomSendErrors[roomId]);
    if (!event.target.value.trim()) {
      setRoomQuickAction(roomId, "");
    }
    updateRoomDraft(roomId, event.target.value);
    delete roomSendErrors[roomId];
    if (hadDraft !== roomHasDraft(roomId) || hadError) {
      renderRooms();
    }
  }
  autoSizeComposerInput();
  renderConversationOverview();
  updateComposerState();
});

cityCreateFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = cityCreateFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitCreateCity();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "创建城市失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

cityJoinFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = cityJoinFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitJoinCity();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "加入城市失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

roomCreateFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = roomCreateFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitCreateRoom();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "创建房间失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

directOpenFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = directOpenFormEl.querySelector("button");
  button.disabled = true;
  try {
    await openDirectSession(directPeerInputEl.value);
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "打开私聊失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

providerConnectFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = providerConnectFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitProviderConnect();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "连接消息来源失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

providerDisconnectButtonEl?.addEventListener("click", async () => {
  providerDisconnectButtonEl.disabled = true;
  try {
    await submitProviderDisconnect();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "断开消息来源失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

// Wire one isolated auth controller to this shell runtime.
const authController = createAuthController({
  statusEl: authStatusEl,
  requestFormEl: authRequestFormEl,
  deliverySelectEl: authDeliverySelectEl,
  residentInputEl: authResidentInputEl,
  nicknameInputEl: authNicknameInputEl,
  nicknameEditorEl: authNicknameEditorEl,
  nicknameEditInputEl: authNicknameEditInputEl,
  nicknameSaveBtnEl: authNicknameSaveBtnEl,
  emailInputEl: authEmailInputEl,
  mobileInputEl: authMobileInputEl,
  deviceInputEl: authDeviceInputEl,
  verifyFormEl: authVerifyFormEl,
  challengeInputEl: authChallengeInputEl,
  codeInputEl: authCodeInputEl,
  loginCardEl: residentLoginCardEl,
  loginOverlayEl: residentLoginOverlayEl,
  hudLoginToggleEl: hudLoginToggleEl,
}, {
  postJson: postGatewayJson,
  postAuthenticated: postGatewayJson,
  refreshFromGateway,
  persistIdentity: persistSenderIdentity,
  onGatewayAuthFailure: () => persistSenderIdentity("访客"),
  userProjection: userShellProjection,
  gatewayUrl: () => gatewayUrl,
  desiredResidentId: () => {
    const value = authResidentInputEl?.value?.trim() || identityInputEl?.value?.trim();
    return value || undefined;
  },
});
const {
  getAuthSession,
  getSessionToken,
  loadAuthDraft: loadAuthDraftMod,
  persistAuthDraft: persistAuthDraftMod,
  residentGatewayLoginRequired: _residentGatewayLoginRequired,
  setAuthStatus: setAuthStatusMod,
  updateAuthFormState: updateAuthFormStateMod,
  updateResidentLoginSurface: applyResidentLoginSurface,
  handleGatewayAuthFailure: handleGatewayAuthFailureMod,
  hasGatewayAuthFailure: hasGatewayAuthFailureMod,
  logout: logoutMod,
  requestEmailOtp: requestEmailOtpMod,
  updateMyNickname: updateMyNicknameMod,
  verifyEmailOtp: verifyEmailOtpMod,
} = authController;

authRequestFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = authRequestFormEl.querySelector("button");
  button.disabled = true;
  persistAuthDraft();
  try {
    await requestEmailOtp();
  } catch (error) {
    setAuthStatus(localizedRuntimeError(error, "申请验证码失败"), true);
  } finally {
    updateAuthFormState();
  }
});

authVerifyFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = authVerifyFormEl.querySelector("button");
  button.disabled = true;
  persistAuthDraft();
  try {
    await verifyEmailOtp();
    residentLoginDismissed = false;
    if (authNicknameEditorEl) authNicknameEditorEl.classList.remove("shell-hidden");
  } catch (error) {
    setAuthStatus(localizedRuntimeError(error, "验证码校验失败"), true);
  } finally {
    updateAuthFormState();
  }
});

hudLoginToggleEl?.addEventListener("click", async () => {
  if (!getSessionToken()) {
    residentLoginDismissed = false;
    updateResidentLoginSurface();
    return;
  }
  hudLoginToggleEl.disabled = true;
  try {
    // 隐私加固：先于会话吊销静默退订本浏览器的推送，防同设备跨居民泄漏
    await pushClientInstance?.disableSilently();
    await logoutMod();
  } catch (error) {
    setAuthStatus(localizedRuntimeError(error, "退出登录失败"), true);
  } finally {
    hudLoginToggleEl.disabled = false;
    residentLoginDismissed = false;
    updateResidentLoginSurface();
  }
});

authNicknameSaveBtnEl?.addEventListener("click", async () => {
  const nickname = authNicknameEditInputEl?.value?.trim() || null;
  authNicknameSaveBtnEl.disabled = true;
  await updateMyNickname(nickname);
  authNicknameSaveBtnEl.disabled = false;
});

residentLoginCloseEl?.addEventListener("click", () => {
  residentLoginDismissed = true;
  updateResidentLoginSurface();
});

worldMirrorFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = worldMirrorFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitAddMirrorSource();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "添加镜像源失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

worldNoticeFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = worldNoticeFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitWorldNotice();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "发布世界公告失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

worldTrustFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = worldTrustFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitCityTrustUpdate();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "更新城市信任状态失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

worldAdvisoryFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = worldAdvisoryFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitWorldAdvisory();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "发布安全通告失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

worldReportReviewFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = worldReportReviewFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitWorldReportReview();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "审查举报失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

worldReportFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = worldReportFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitWorldReport();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "提交举报失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

worldResidentSanctionFormEl?.addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = worldResidentSanctionFormEl.querySelector("button");
  button.disabled = true;
  try {
    await submitResidentSanction();
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "发布居民制裁失败"), true);
  } finally {
    updateGovernanceFormState();
  }
});

exportCurrentButtonEl?.addEventListener("click", async () => {
  await exportCurrentConversation("导出当前会话失败");
});

exportAllButtonEl?.addEventListener("click", async () => {
  try {
    await exportHistory({ conversationId: null, includePublic: true });
  } catch (error) {
    setGovernanceStatus(localizedRuntimeError(error, "导出全部历史失败"), true);
  }
});

identityInputEl?.addEventListener("change", async (event) => {
  persistSenderIdentity(event.target.value);
  await refreshIdentityProjection();
});

identityInputEl?.addEventListener("blur", async (event) => {
  persistSenderIdentity(event.target.value);
  await refreshIdentityProjection();
});

// WeChat drawer toggle for user page
const railDrawerEl = document.querySelector("#rail-drawer");
const railToggleEl = document.querySelector("#rail-toggle");
const drawerCloseEl = document.querySelector("#drawer-close");

if (railToggleEl && railDrawerEl) {
  railToggleEl.addEventListener("click", () => {
    railDrawerEl.classList.toggle("open");
  });
}

if (drawerCloseEl && railDrawerEl) {
  drawerCloseEl.addEventListener("click", () => {
    railDrawerEl.classList.remove("open");
  });
}

// Hub page rail toggle for mobile
const hudRailToggleEl = document.querySelector("#hud-rail-toggle");
const sfcRailEl = document.querySelector(".sfc-rail");

function setSfcRailOpen(open) {
  if (!sfcRailEl) return;
  sfcRailEl.classList.toggle("open", open);
  sfcRailEl.setAttribute("aria-hidden", open ? "false" : "true");
  if (hudRailToggleEl) {
    hudRailToggleEl.setAttribute("aria-expanded", open ? "true" : "false");
  }
  document.body.classList.toggle("rail-drawer-open", open);
}

if (hudRailToggleEl && sfcRailEl) {
  hudRailToggleEl.addEventListener("click", () => {
    setSfcRailOpen(!sfcRailEl.classList.contains("open"));
  });
}

// escapeHtml moved to shell-message-render.js

const composerSymbolController = createComposerSymbolController({
  doc: document,
  inputEl: composerInputEl,
  mentionTriggerEl: composerMentionTriggerEl,
  symbolTriggerEl: composerSymbolTriggerEl,
  symbolMenuEl: composerSymbolMenuEl,
  symbolInsertEls: composerSymbolInsertEls,
});
composerSymbolController.bind();

const sceneRuntime = initSceneRuntime({
  onEscape: composerSymbolController.close,
  isRailOpen: () => Boolean(sfcRailEl?.classList.contains("open")),
  closeRail: () => setSfcRailOpen(false),
});
sceneRuntime.bindTimeline(timelineEl);

// shell-composer 模块依赖注入：DOM 元素/对象引用直接传，会变的原始值用 getter
// 保证模块内 _ctx 读取的是当前值而非初始化快照。
function buildComposerDeps() {
  return {
    get composerFormEl() { return composerFormEl; },
    get composerInputEl() { return composerInputEl; },
    get composerHeroEl() { return composerHeroEl; },
    get composerMetaEl() { return composerMetaEl; },
    get composerContextEl() { return composerContextEl; },
    get composerStatusEl() { return composerStatusEl; },
    get composerTipEl() { return composerTipEl; },
    set composerTipEl(v) { composerTipEl = v; },
    get activeRoomId() { return activeRoomId; },
    get shellMode() { return shellMode; },
    get gatewayUrl() { return gatewayUrl; },
    get isSendingMessage() { return messageSendInFlight(); },
    get refreshInProgress() { return gatewaySyncController.isRefreshing(); },
    get lastRefreshErrorMessage() { return gatewaySyncController.lastErrorMessage(); },
    get providerLoaded() { return providerLoaded; },
    get provider() { return provider; },
    get lastSentMessage() { return lastSentMessage; },
    get lastComposerKeyboardSubmitAt() { return lastComposerKeyboardSubmitAt; },
    set lastComposerKeyboardSubmitAt(v) { lastComposerKeyboardSubmitAt = v; },
    get state() { return state; },
    get roomSendErrors() { return roomSendErrors; },
    get roomReadMarkers() { return roomReadMarkers; },
    currentIdentity,
    quickActionTemplate,
    roomQuickAction,
    setRoomQuickAction,
    draftForRoom,
    roomHasDraft,
    visiblePendingEchoCount,
    roomThreadHeadline,
    roomDisplayPeer,
    roomAudienceLabel,
    roomRouteLabel,
    roomChatStatusSummary,
    roomQueueSummary,
    roomSyncLabel,
    updateComposerState,
    renderConversationOverview,
    submitComposerMessage,
  };
}
initShellComposer(buildComposerDeps());

function renderSceneHotspotsForRoom(room) {
  sceneRuntime.renderSceneHotspotsForRoom(room);
}

// Close drawer when clicking outside
document.addEventListener("click", (event) => {
  if (railDrawerEl?.classList.contains("open") &&
      !railDrawerEl.contains(event.target) &&
      !railToggleEl?.contains(event.target)) {
    railDrawerEl.classList.remove("open");
  }
  if (sfcRailEl?.classList.contains("open") &&
      !sfcRailEl.contains(event.target) &&
      !hudRailToggleEl?.contains(event.target)) {
    setSfcRailOpen(false);
  }
});

initRail(
  {
    roomListEl,
    residentListEl,
    roomSearchInputEl,
    roomToolbarNoteEl,
    roomFilterButtons,
    conversationOverviewEl,
    roomDigestEl,
  },
  {
    getRooms: () => state.rooms,
    getActiveRoomId: () => activeRoomId,
    setActiveRoomId: (id) => { activeRoomId = id; },
    getRoomFilter: () => roomFilter,
    setRoomFilter: (f) => { roomFilter = f; },
    getRoomSearch: () => roomSearch,
    setRoomSearch: (s) => { roomSearch = s; },
    getGatewayUrl: () => gatewayUrl,
    getShellPage: () => currentShellPage(),
    getCurrentIdentity: () => currentIdentity(),
    getRoomReadMarkers: () => roomReadMarkers,
    persistRoomReadMarkers: () => persistRoomReadMarkers(),
    createLine,
    createPill,
    clearChildren,
    translateRoomKind,
    translateRoomKindForShellPage,
    roomHasDraft,
    visiblePendingEchoCount,
    visiblePendingEchoesForRoom,
    roomSendErrors: () => roomSendErrors,
    roomSyncLabel,
    latestRoomQuickAction,
    roomQuickActionSummary,
    resolveRoomQuickPreview,
    createRoomQuickActionPill,
    createRoomQuickPreviewPill,
    createRoomInlineActions,
    ensureRoomQuickActions,
    caretakerProfile,
    caretakerPendingCount,
    joinOrFallback,
    roomLastActivity,
    roomRouteLabel,
  },
);

registerUnhandledRuntimeReporter();
main();
