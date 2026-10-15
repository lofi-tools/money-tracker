const field = document.querySelector("#connection");
field.value =
  (await chrome.storage.local.get("connectionId")).connectionId ?? "";
document.querySelector("#enable").onclick = async () => {
  const id = field.value.trim();
  if (!/^[a-f0-9]{64}$/.test(id)) {
    document.querySelector("#status").textContent =
      "Enter the 64-character connection ID from local tracker setup.";
    return;
  }
  const granted = await chrome.permissions.request({
    origins: ["https://platform.nexo.com/*"],
  });
  if (!granted) return;
  await chrome.storage.local.set({ connectionId: id });
  await chrome.runtime.sendMessage({ type: "configure" });
  document.querySelector("#status").textContent =
    "Enabled. Reload Nexo; the floating export button appears while signed in.";
};
document.querySelector("#disable").onclick = async () => {
  await chrome.permissions.remove({ origins: ["https://platform.nexo.com/*"] });
  await chrome.scripting
    .unregisterContentScripts({ ids: ["nexo-capture"] })
    .catch(() => {});
  await chrome.storage.session.clear();
  document.querySelector("#status").textContent =
    "Disabled. Reload Nexo to remove the control.";
};

document.querySelector("#retry").onclick = async () => {
  const result = await chrome.runtime.sendMessage({ type: "retry" });
  document.querySelector("#status").textContent =
    result.status === "committed"
      ? `Imported: ${result.receipt.matched} matched, ${result.receipt.unresolved} unresolved`
      : (result.message ?? "Received; ingestion still pending");
};
