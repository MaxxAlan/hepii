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

  // Các nút hiện có (giữ nguyên)
  document.getElementById("loadLibrary").addEventListener("click", async () => {
    const success = await executeScript("TV.js");
    if (success) {
      await delay(1000);
      alert("Đã nhúng thư viện thành công!");
    }
  });

  document.getElementById("getExamCode").addEventListener("click", async () => {
    const success = await executeScript("getexamcode.js");
  });

  document.getElementById("getAnswer").addEventListener("click", async () => {
    const success = await executeScript("getanswer.js");
  });

  document.getElementById("downloadImages").addEventListener("click", async () => {
    await executeScript("TV.js");
    await delay(500);
    const success = await executeScript("img.js");
    if (success) {
      alert("Đã chạy script tải ảnh!");
    }
  });

  document.getElementById("downloadText").addEventListener("click", async () => {
    await executeScript("TV.js");
    await delay(500);
    const success = await executeScript("img_text.js");
    if (success) {
      alert("Đã chạy script tải text!");
    }
  });

  // Thêm logic cho tự động xóa dữ liệu
  const codeListInput = document.getElementById("codeList");
  const delayTimeInput = document.getElementById("delayTime");
  const startButton = document.getElementById("startAutoDelete");
  const stopButton = document.getElementById("stopAutoDelete");
  const statusMessage = document.getElementById("statusMessage");
  const progressInfo = document.getElementById("progressInfo");
  const statusBox = document.getElementById("statusBox");

  // Hàm cập nhật giao diện
  function updateUI(data) {
    if (data.isRunning) {
      statusMessage.textContent = `Trạng thái: Đang chạy (Mã: ${data.codeList[data.currentIndex]})`;
      progressInfo.textContent = `Tiến độ: ${data.currentIndex + 1}/${data.totalCodes}`;
      statusBox.className = "status-box running";
      startButton.disabled = true;
      stopButton.disabled = false;
    } else {
      const displayIndex = data.currentIndex >= data.totalCodes ? data.totalCodes : data.currentIndex;
      statusMessage.textContent = "Trạng thái: Đã dừng";
      progressInfo.textContent = `Tiến độ: ${displayIndex}/${data.totalCodes}`;
      statusBox.className = "status-box";
      startButton.disabled = false;
      stopButton.disabled = true;
    }
  }

  // Hàm đếm số lượng mã hợp lệ từ textarea
  function countValidCodes() {
    const codeList = codeListInput.value.split("\n").filter((code) => code.trim() !== "");
    return codeList.length;
  }

  // Cập nhật giao diện khi người dùng nhập mã
  codeListInput.addEventListener("input", () => {
    const totalCodes = countValidCodes();
    updateUI({
      isRunning: false,
      currentIndex: 0,
      totalCodes: totalCodes,
      codeList: codeListInput.value.split("\n").filter((code) => code.trim() !== ""),
    });
  });

  // Kiểm tra trạng thái ban đầu
  chrome.storage.local.get(["codeList", "currentIndex", "totalCodes", "isRunning"], (data) => {
    if (data.codeList) {
      codeListInput.value = data.codeList.join("\n");
      updateUI(data);
    } else {
      updateUI({ isRunning: false, currentIndex: 0, totalCodes: 0 });
    }
  });

  // Lắng nghe thông điệp từ autodelete.js để cập nhật trạng thái
  chrome.runtime.onMessage.addListener((message) => {
    if (message.action === "updateStatus") {
      // Sử dụng dữ liệu từ message thay vì gọi lại chrome.storage.local.get
      const data = {
        currentIndex: message.currentIndex,
        totalCodes: message.totalCodes,
        codeList: message.codeList,
        isRunning: message.isRunning,
      };
      updateUI(data);
      if (message.status === "success") {
        statusBox.className = "status-box success";
        statusMessage.textContent = `Trạng thái: ${message.message}`;
      } else if (message.status === "error") {
        statusBox.className = "status-box error";
        statusMessage.textContent = `Trạng thái: ${message.message}`;
      }
    }
  });

  // Bắt đầu tự động xóa
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

    chrome.tabs.create({ url: "https://app.kizspy.me/support/input" });
  });

  // Dừng tự động xóa
  stopButton.addEventListener("click", async () => {
    await chrome.storage.local.set({ isRunning: false });
    chrome.storage.local.get(["codeList", "currentIndex", "totalCodes"], (data) => {
      updateUI({ ...data, isRunning: false });
    });
  });
});