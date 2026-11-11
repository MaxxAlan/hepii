chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.status === "complete" && tab.url.includes("app.kizspy.me")) {
    chrome.storage.local.get(["isRunning"], function (data) {
      if (data.isRunning) {
        chrome.scripting.executeScript({
          target: { tabId: tabId },
          files: ["scripts/autodelete.js"],
        })
          .then(() => {
            chrome.tabs.sendMessage(tabId, { action: "processCurrentCode" });
          })
          .catch((err) => {
            console.error("Lỗi khi thực thi script:", err);
          });
      }
    });
  }
});