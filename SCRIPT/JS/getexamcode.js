(async function () {
  // Hàm để thiết lập XHR/Fetch Interceptor và lấy examCode từ các request watcher
  function setupNetworkInterceptor() {
    return new Promise((resolve) => {
      // Tạo script để chèn vào trang
      const script = document.createElement("script");
      // Load the interceptor script from the extension's web-accessible resources
      script.src = chrome.runtime.getURL("scripts/interceptor.js");
      script.onload = () => {
        console.log("Interceptor script loaded successfully");
      };
      script.onerror = (error) => {
        console.error("Failed to load interceptor script:", error);
        resolve(null); // Resolve with null if the script fails to load
      };

      document.head.appendChild(script);

      // Lắng nghe message từ script
      const messageHandler = (event) => {
        if (event.data && event.data.type === "EXAM_CODE_FOUND") {
          window.removeEventListener("message", messageHandler);
          resolve(event.data.examCode);
        }
      };

      window.addEventListener("message", messageHandler);

      // Timeout sau 10 giây nếu không tìm thấy
      setTimeout(() => {
        window.removeEventListener("message", messageHandler);
        resolve(null);
      }, 10000);
    });
  }

  // Tạo popup để hiển thị kết quả
  function createPopup(examCode) {
    // Xóa popup cũ nếu có
    const existingPopup = document.getElementById("exam-code-popup");
    if (existingPopup) {
      existingPopup.remove();
    }

    // Tạo và style popup
    const popup = document.createElement("div");
    popup.id = "exam-code-popup";
    popup.style.cssText = `
      position: fixed;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%);
      background: white;
      padding: 20px;
      border-radius: 8px;
      box-shadow: 0 0 10px rgba(0,0,0,0.3);
      z-index: 10000;
      min-width: 300px;
      max-width: 80%;
      font-family: Arial, sans-serif;
    `;

    // Tạo nội dung popup
    if (examCode) {
      popup.innerHTML = `
        <h3 style="margin-top: 0; margin-bottom: 15px;">Exam Code</h3>
        <div style="display: flex; margin-bottom: 15px;">
          <input type="text" value="${examCode}" readonly style="flex: 1; padding: 8px; margin-right: 10px;">
          <button id="copyExamCode" style="padding: 8px 15px; cursor: pointer;">Copy</button>
        </div>
        <button id="closePopup" style="width: 100%; padding: 8px 15px; cursor: pointer;">Đóng</button>
      `;
    } else {
      popup.innerHTML = `
        <h3 style="margin-top: 0; margin-bottom: 15px;">Thông báo</h3>
        <p style="margin-bottom: 15px;">Không tìm thấy Exam Code. Vui lòng làm mới trang và thử lại.</p>
        <button id="closePopup" style="width: 100%; padding: 8px 15px; cursor: pointer;">Đóng</button>
      `;
    }

    // Thêm popup vào trang
    document.body.appendChild(popup);

    // Xử lý sự kiện
    if (examCode) {
      document.getElementById("copyExamCode").addEventListener("click", () => {
        const input = popup.querySelector("input");
        input.select();
        document.execCommand("copy");
        alert("Đã copy Exam Code!");
      });
    }

    // Xử lý sự kiện nút Đóng
    document.getElementById("closePopup").onclick = function () {
      popup.remove();
    };
  }

  // Thực thi chính
  try {
    console.log("Bắt đầu tìm kiếm Exam Code từ các request watcher...");

    // Thiết lập interceptor để lấy examCode từ các request watcher
    const examCode = await setupNetworkInterceptor();
    console.log("Exam Code tìm thấy:", examCode);

    // Hiển thị kết quả
    createPopup(examCode);
  } catch (error) {
    console.error("Lỗi khi lấy Exam Code:", error);
    createPopup(null); // Hiển thị thông báo lỗi
  }
})();