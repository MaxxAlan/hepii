// Lưu lại XHR gốc
const originalXHR = window.XMLHttpRequest;

// Ghi đè XHR để theo dõi
window.XMLHttpRequest = function () {
  const xhr = new originalXHR();
  const originalOpen = xhr.open;
  const originalSend = xhr.send;

  xhr.open = function () {
    this._url = arguments[1];
    return originalOpen.apply(this, arguments);
  };

  xhr.send = function () {
    if (this._url && this._url.includes("/watcher")) {
      this.addEventListener("load", function () {
        try {
          if (this.responseText) {
            const response = JSON.parse(this.responseText);
            let examCode = null;

            if (response && response.doing) {
              examCode = response.doing;
            } else if (response && response.Information && response.Information.doing) {
              examCode = response.Information.doing;
            } else if (response && response.information && response.information.doing) {
              examCode = response.information.doing;
            }

            if (examCode) {
              window.postMessage(
                {
                  type: "EXAM_CODE_FOUND",
                  examCode: examCode,
                },
                window.location.origin
              );
            }
          }
        } catch (e) {
          console.error("Lỗi khi xử lý XHR response:", e);
        }
      });
    }
    return originalSend.apply(this, arguments);
  };

  return xhr;
};

// Lưu lại fetch gốc
const originalFetch = window.fetch;

// Ghi đè fetch để theo dõi
window.fetch = async function (url, options) {
  if (url && url.toString().includes("/watcher")) {
    const response = await originalFetch(url, options);
    const clone = response.clone();

    clone
      .json()
      .then((data) => {
        let examCode = null;

        if (data && data.doing) {
          examCode = data.doing;
        } else if (data && data.Information && data.Information.doing) {
          examCode = data.Information.doing;
        } else if (data && data.information && data.information.doing) {
          examCode = data.information.doing;
        }

        if (examCode) {
          window.postMessage(
            {
              type: "EXAM_CODE_FOUND",
              examCode: examCode,
            },
            window.location.origin
          );
        }
      })
      .catch((e) => console.error("Lỗi khi xử lý fetch response:", e));

    return response;
  }

  return originalFetch(url, options);
};