// Owns one shell instance's send-in-flight state and transport ordering.
// Canonical message state still comes from the Gateway refresh payload.

function errorFrom(value, fallback) {
  if (value instanceof Error) return value;
  return new Error(typeof value === "string" && value ? value : fallback);
}

export function createMessageSendController({
  getContext = () => ({ roomId: "", gatewayConnected: false, loginRequired: false }),
  commitLocal = () => false,
  buildPayload = () => ({}),
  prepareGateway = () => null,
  postGateway = async () => {},
  clearSendError = () => {},
  refreshGateway = async () => {},
  clearPending = () => {},
  handleFailure = ({ error }) => error,
  onSettled = () => {},
} = {}) {
  let sending = false;

  function isSending() {
    return sending;
  }

  async function send(text, { quickAction = "", attachmentId = "" } = {}) {
    const context = getContext() || {};
    const roomId = typeof context.roomId === "string" ? context.roomId : "";
    if (!roomId || sending) return false;
    if (context.loginRequired) throw new Error("请先登录后发送");

    const request = { roomId, text, quickAction, attachmentId };
    if (!context.gatewayConnected) {
      return Boolean(await commitLocal(request));
    }

    sending = true;
    let pendingEchoId = null;
    let posted = false;
    try {
      const payload = buildPayload(request);
      pendingEchoId = prepareGateway(request);
      await postGateway({ ...request, payload });
      posted = true;
      clearSendError({ roomId });
      await refreshGateway({ roomId });
      clearPending({ roomId });
      return true;
    } catch (error) {
      throw errorFrom(
        handleFailure({ roomId, pendingEchoId, posted, error }),
        posted ? "消息可能已发出，但会话同步失败" : "消息发送失败",
      );
    } finally {
      sending = false;
      onSettled();
    }
  }

  return { isSending, send };
}
