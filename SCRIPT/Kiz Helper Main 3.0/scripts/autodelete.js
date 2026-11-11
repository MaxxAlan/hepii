chrome.runtime.onMessage.addListener(function (request, sender, sendResponse) {
  if (request.action === "processCurrentCode") {
    processCurrentCode();
  }
});

async function processCurrentCode() {
  try {
    const data = await chrome.storage.local.get(["codeList", "currentIndex", "delayTime", "totalCodes", "isRunning"]);

    if (!data.isRunning) {
      sendStatusUpdate("Đã dừng bởi người dùng", "success", data);
      return;
    }

    const currentCode = data.codeList[data.currentIndex];
    console.log(`Đang xử lý mã: ${currentCode} (${data.currentIndex + 1}/${data.totalCodes})`);

    sendStatusUpdate(`Đang xử lý mã: ${currentCode}`, "running", data);

    if (window.location.href.includes("/support/input")) {
      const captchaElement = document.querySelector(".g-recaptcha") || document.querySelector("#cf-error-details");
      if (captchaElement) {
        throw new Error("Trang yêu cầu nhập CAPTCHA hoặc xác nhận bảo mật. Vui lòng xử lý thủ công trước khi tiếp tục.");
      }

      await handleLoginPage(currentCode);
    } else if (document.querySelector(".swal2-popup")) {
      await handleDeleteDialog();
    } else {
      console.log("Đang ở trang khác, đợi hộp thoại xuất hiện...");
      await new Promise((resolve) => setTimeout(resolve, 5000));
      if (document.querySelector(".swal2-popup")) {
        await handleDeleteDialog();
      } else {
        moveToNextCode("Không tìm thấy hộp thoại xóa sau khi đăng nhập", "error");
      }
    }
  } catch (error) {
    console.error("Lỗi khi xử lý mã:", error);
    moveToNextCode(`Lỗi: ${error.message}`, "error");
  }
}

function sendStatusUpdate(message, status, data) {
  chrome.runtime.sendMessage({
    action: "updateStatus",
    message,
    status,
    currentIndex: data.currentIndex,
    totalCodes: data.totalCodes,
    codeList: data.codeList,
    isRunning: data.isRunning,
  });
}

async function handleLoginPage(code) {
  await waitForElement('input[name="support_code"]', 15000);

  const inputField = document.querySelector('input[name="support_code"]');
  if (!inputField) {
    throw new Error("Không tìm thấy ô nhập mã");
  }

  inputField.value = code;
  inputField.dispatchEvent(new Event("input", { bubbles: true }));
  console.log(`Đã điền mã: ${code}`);

  const submitButton = document.getElementById("kt_password_reset_submit");
  if (!submitButton) {
    const possibleButtons = Array.from(document.querySelectorAll("button")).filter(
      (button) => button.textContent.includes("Xác nhận")
    );

    if (possibleButtons.length === 0) {
      throw new Error("Không tìm thấy nút Xác nhận");
    }

    possibleButtons[0].click();
  } else {
    if (submitButton.disabled) {
      throw new Error("Nút Xác nhận đang bị vô hiệu hóa. Có thể trang đang xử lý hoặc có lỗi.");
    }

    const clickEvent = new Event("click", { bubbles: true, cancelable: true });
    const mouseDownEvent = new Event("mousedown", { bubbles: true });
    const mouseUpEvent = new Event("mouseup", { bubbles: true });

    submitButton.dispatchEvent(mouseDownEvent);
    submitButton.dispatchEvent(clickEvent);
    submitButton.dispatchEvent(mouseUpEvent);
    console.log("Đã nhấn nút Xác nhận");
  }

  try {
    await waitForElement(".swal2-popup", 20000);
    console.log("Hộp thoại xóa đã xuất hiện");
    await handleDeleteDialog();
  } catch (error) {
    const errorElement = document.querySelector(".invalid-feedback");
    if (errorElement && errorElement.style.display !== "none") {
      throw new Error(`Lỗi đăng nhập: ${errorElement.textContent.trim()}`);
    }
    throw new Error("Không hiển thị hộp thoại xóa sau khi nhấn Xác nhận: " + error.message);
  }
}

async function handleDeleteDialog() {
  try {
    if (!document.querySelector(".swal2-popup")) {
      throw new Error("Hộp thoại không còn tồn tại");
    }

    const deleteButton = document.querySelector(".swal2-confirm");
    if (!deleteButton) {
      throw new Error("Không tìm thấy nút Xóa trong hộp thoại");
    }

    if (deleteButton.disabled) {
      throw new Error("Nút Xóa đang bị vô hiệu hóa");
    }

    const clickEvent = new Event("click", { bubbles: true, cancelable: true });
    const mouseDownEvent = new Event("mousedown", { bubbles: true });
    const mouseUpEvent = new Event("mouseup", { bubbles: true });

    deleteButton.dispatchEvent(mouseDownEvent);
    deleteButton.dispatchEvent(clickEvent);
    deleteButton.dispatchEvent(mouseUpEvent);
    console.log("Đã nhấn nút Xóa");

    let attempts = 0;
    const maxAttempts = 25;
    while (document.querySelector(".swal2-popup") && attempts < maxAttempts) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      attempts++;
    }

    if (document.querySelector(".swal2-popup")) {
      throw new Error("Hộp thoại không đóng sau khi nhấn Xóa. Có thể hành động xóa thất bại.");
    }

    console.log("Hộp thoại đã đóng, xóa thành công");
    moveToNextCode("Đã xóa dữ liệu thành công", "success");
  } catch (error) {
    console.error("Lỗi khi xử lý hộp thoại xóa:", error);
    if (!document.querySelector(".swal2-popup")) {
      moveToNextCode("Hộp thoại đã đóng nhưng có lỗi: " + error.message, "error");
    } else {
      moveToNextCode("Không thể xóa: " + error.message, "error");
    }
  }
}

function waitForElement(selector, timeout = 10000) {
  return new Promise((resolve, reject) => {
    if (document.querySelector(selector)) {
      return resolve(document.querySelector(selector));
    }

    const observer = new MutationObserver((mutations) => {
      if (document.querySelector(selector)) {
        observer.disconnect();
        resolve(document.querySelector(selector));
      }
    });

    observer.observe(document.body, {
      childList: true,
      subtree: true,
    });

    setTimeout(() => {
      observer.disconnect();
      reject(new Error(`Timeout: Không tìm thấy phần tử ${selector} sau ${timeout}ms`));
    }, timeout);
  });
}

async function moveToNextCode(message, status) {
  const data = await chrome.storage.local.get(["currentIndex", "codeList", "delayTime", "totalCodes", "isRunning"]);

  if (!data.isRunning) {
    sendStatusUpdate("Đã dừng bởi người dùng", "success", data);
    return;
  }

  const nextIndex = data.currentIndex + 1;

  // Cập nhật currentIndex ngay cả khi đã xử lý mã cuối cùng
  await chrome.storage.local.set({ currentIndex: nextIndex });

  // Tạo dữ liệu mới để gửi qua message
  const updatedData = {
    currentIndex: nextIndex,
    totalCodes: data.totalCodes,
    codeList: data.codeList,
    isRunning: data.isRunning,
  };

  sendStatusUpdate(message, status, updatedData);

  if (nextIndex < data.codeList.length) {
    console.log(`${message}. Chuyển sang mã tiếp theo sau ${data.delayTime / 1000}s`);

    setTimeout(() => {
      window.location.href = "https://app.kizspy.me/support/input";
    }, data.delayTime);
  } else {
    console.log("Đã hoàn thành tất cả mã!");
    await chrome.storage.local.set({ isRunning: false });
    updatedData.isRunning = false; // Cập nhật trạng thái isRunning
    sendStatusUpdate("Đã hoàn thành tất cả mã!", "success", updatedData);
  }
}