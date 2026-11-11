document.addEventListener("DOMContentLoaded", function () {
  async function executeScript(scriptName) {
    try {
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      await chrome.scripting.executeScript({
        target: { tabId: tab.id },
        files: [`scripts/${scriptName}`],
      });
      return true;
    } catch (err) {
      console.error(`Error executing ${scriptName}:`, err);
      alert(`Lỗi khi chạy ${scriptName}: ${err.message}`);
      return false;
    }
  }

  const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

  // Các nút hiện có
  document.getElementById("loadLibrary").addEventListener("click", async () => {
    const success = await executeScript("TV.js");
    if (success) {
      await delay(1000);
      alert("Đã nhúng thư viện thành công!");
    }
  });

  document.getElementById("getExamCode").addEventListener("click", async () => {
    await executeScript("getexamcode.js");
  });

  document.getElementById("getAnswer").addEventListener("click", async () => {
    await executeScript("getanswer.js");
  });

  document.getElementById("downloadImages").addEventListener("click", async () => {
    // Đảm bảo thư viện đã được load
    const tvSuccess = await executeScript("TV.js");
    if (!tvSuccess) {
      alert("Không thể nhúng thư viện TV.js. Vui lòng thử lại!");
      return;
    }

    // Tăng thời gian delay để đảm bảo TV.js khởi tạo xong
    await delay(1500); // Tăng từ 500ms lên 1500ms

    // Kiểm tra xem TV.js đã khởi tạo xong chưa (giả sử TV.js tạo biến window.TV)
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    const tvCheck = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: () => typeof window.TV !== "undefined",
    });

    if (!tvCheck[0].result) {
      alert("Thư viện TV.js chưa sẵn sàng. Vui lòng thử lại hoặc liên hệ hỗ trợ!");
      return;
    }

    const success = await executeScript("img.js");
    if (success) {
      alert("Đã chạy script tải ảnh!");
    } else {
      alert("Không thể chạy script tải ảnh. Vui lòng kiểm tra console để biết thêm chi tiết!");
    }
  });

  document.getElementById("downloadText").addEventListener("click", async () => {
    const tvSuccess = await executeScript("TV.js");
    if (!tvSuccess) {
      alert("Không thể nhúng thư viện TV.js. Vui lòng thử lại!");
      return;
    }

    await delay(1500); // Tăng từ 500ms lên 1500ms

    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    const tvCheck = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: () => typeof window.TV !== "undefined",
    });

    if (!tvCheck[0].result) {
      alert("Thư viện TV.js chưa sẵn sàng. Vui lòng thử lại hoặc liên hệ hỗ trợ!");
      return;
    }

    const success = await executeScript("img_text.js");
    if (success) {
      alert("Đã chạy script tải text!");
    }
  });

  // Tự động xóa dữ liệu (giữ nguyên phần này)
  const codeListInput = document.getElementById("codeList");
  const delayTimeInput = document.getElementById("delayTime");
  const startButton = document.getElementById("startAutoDelete");
  const stopButton = document.getElementById("stopAutoDelete");
  const clearButton = document.getElementById("clearCodeList");
  const statusMessage = document.getElementById("statusMessage");
  const progressInfo = document.getElementById("progressInfo");
  const statusBox = document.getElementById("statusBox");

  function updateUI(data) {
    if (data.isRunning) {
      statusMessage.textContent = `Trạng thái: Đang chạy (Mã: ${data.codeList[data.currentIndex]})`;
      progressInfo.textContent = `Tiến độ: ${data.currentIndex + 1}/${data.totalCodes}`;
      statusBox.className = "status-box running";
      startButton.disabled = true;
      stopButton.disabled = false;
    } else {
      statusMessage.textContent = "Trạng thái: Đã dừng";
      progressInfo.textContent = `Tiến độ: ${data.currentIndex}/${data.totalCodes}`;
      statusBox.className = "status-box";
      startButton.disabled = false;
      stopButton.disabled = true;
    }
  }

  chrome.storage.local.get(["codeList", "currentIndex", "totalCodes", "isRunning"], (data) => {
    if (data.codeList) {
      codeListInput.value = data.codeList.join("\n");
      updateUI(data);
    } else {
      updateUI({ isRunning: false, currentIndex: 0, totalCodes: 0 });
    }
  });

  chrome.runtime.onMessage.addListener((message) => {
    if (message.action === "updateStatus") {
      chrome.storage.local.get(["codeList", "currentIndex", "totalCodes", "isRunning"], (data) => {
        updateUI(data);
        if (message.status === "success") {
          statusBox.className = "status-box success";
          statusMessage.textContent = `Trạng thái: ${message.message}`;
        } else if (message.status === "error") {
          statusBox.className = "status-box error";
          statusMessage.textContent = `Trạng thái: ${message.message}`;
        }
      });
    }
  });

  startButton.addEventListener("click", async () => {
    const codeList = codeListInput.value.split("\n").filter((code) => code.trim() !== "");
    const delayTime = parseInt(delayTimeInput.value) * 1000;

    if (codeList.length === 0) {
      alert("Vui lòng nhập ít nhất một mã!");
      return;
    }

    if (delayTime < 2000 || delayTime > 30000) {
      alert("Thời gian chờ phải từ 2 đến 30 giây!");
      return;
    }

    await chrome.storage.local.set({
      codeList: codeList,
      currentIndex: 0,
      totalCodes: codeList.length,
      delayTime: delayTime,
      isRunning: true,
    });

    updateUI({ codeList, currentIndex: 0, totalCodes: codeList.length, isRunning: true });

    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab.url.includes("app.kizspy.me/support/input")) {
      await chrome.tabs.update(tab.id, { url: "https://app.kizspy.me/support/input" });
    } else {
      chrome.tabs.sendMessage(tab.id, { action: "processCurrentCode" });
    }
  });

  stopButton.addEventListener("click", async () => {
    await chrome.storage.local.set({ isRunning: false });
    chrome.storage.local.get(["codeList", "currentIndex", "totalCodes"], (data) => {
      updateUI({ ...data, isRunning: false });
    });
  });

  clearButton.addEventListener("click", async () => {
    await chrome.storage.local.clear();
    codeListInput.value = "";
    updateUI({ isRunning: false, currentIndex: 0, totalCodes: 0 });
  });
});