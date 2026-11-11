document.addEventListener('DOMContentLoaded', function() {
  console.log('🚀 Popup loaded');
  
  const checkBtn = document.getElementById('checkBtn');
  const downloadBtn = document.getElementById('downloadBtn');
  const statusDiv = document.getElementById('status');
  const questionCountDiv = document.getElementById('questionCount');
  
  let isConnected = false;
  let currentTab = null;

  console.log('🔍 Elements found:', {
    checkBtn: !!checkBtn,
    downloadBtn: !!downloadBtn,
    statusDiv: !!statusDiv,
    questionCountDiv: !!questionCountDiv
  });

  initializePopup();

  async function initializePopup() {
    try {
      updateStatus('Đang kết nối với trang...', 'info');
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      currentTab = tab;
      
      if (!tab || !tab.url) throw new Error('Không thể truy cập tab hiện tại');
      if (!tab.url.startsWith('http://') && !tab.url.startsWith('https://')) 
        throw new Error('Extension chỉ hoạt động trên trang web HTTP/HTTPS');
      
      await testConnection();
    } catch (error) {
      console.error('❌ Initialization error:', error);
      updateStatus(`❌ ${error.message}`, 'error');
      showReloadSuggestion();
    }
  }

  async function testConnection() {
    try {
      const response = await sendMessageWithTimeout({ action: 'CHECK_STATUS' }, 5000);
      if (response && response.success) {
        isConnected = true;
        updateStatus(response.message || 'Kết nối thành công', 'success');
        enableButtons();
      } else throw new Error('Content script chưa sẵn sàng');
    } catch (error) {
      console.error('❌ Connection test failed:', error);
      isConnected = false;
      try {
        await injectContentScript();
        setTimeout(testConnection, 2000);
      } catch (injectError) {
        console.error('❌ Cannot inject content script:', injectError);
        updateStatus('❌ Không thể kết nối với trang. Vui lòng refresh trang.', 'error');
      }
    }
  }

  async function injectContentScript() {
    await chrome.scripting.executeScript({ target: { tabId: currentTab.id }, files: ['jszip.min.js'] });
    await chrome.scripting.executeScript({ target: { tabId: currentTab.id }, files: ['content.js'] });
  }

  async function sendMessageWithTimeout(message, timeout = 5000) {
    return new Promise((resolve) => {
      chrome.tabs.sendMessage(currentTab.id, message, (response) => {
        resolve(response);
      });
      setTimeout(() => resolve({ success: false, error: 'Timeout' }), timeout);
    });
  }

  function showReloadSuggestion() {
    if (statusDiv) statusDiv.innerHTML += '<br>Vui lòng làm mới trang và thử lại.';
  }

  if (checkBtn) {
    checkBtn.addEventListener('click', async () => {
      if (!isConnected) {
        updateStatus('❌ Chưa kết nối với trang', 'error');
        return;
      }
      checkBtn.disabled = true;
      try {
        const response = await sendMessageWithTimeout({ action: 'checkQuestions' });
        if (response.success) {
          const count = response.totalQuestions;
          questionCountDiv.textContent = `Tìm thấy ${count} câu hỏi`;
          questionCountDiv.className = count > 0 ? 'question-count success' : 'question-count warning';
          if (count > 0) downloadBtn.disabled = false;
        } else throw new Error(response.error);
      } catch (error) {
        console.error('❌ Check error:', error);
        updateStatus(`❌ Lỗi: ${error.message}`, 'error');
        questionCountDiv.textContent = '';
      } finally {
        checkBtn.disabled = false;
      }
    });
  }

  if (downloadBtn) {
    downloadBtn.addEventListener('click', async (event) => {
      event.preventDefault();
      event.stopPropagation();
      if (!isConnected || downloadBtn.disabled) return;
      disableButtons();
      updateStatus('🚀 Đang chuẩn bị tải xuống...', 'info');
      try {
        const response = await sendMessageWithTimeout({ action: 'downloadAllImages' });
        if (response.success) {
          updateStatus(response.message, 'success');
        } else throw new Error(response.error);
      } catch (error) {
        console.error('❌ Download error:', error);
        updateStatus(`❌ Lỗi: ${error.message}`, 'error');
      } finally {
        enableButtons();
      }
    });
  }

  function updateStatus(message, type = 'info') {
    console.log(`📊 Status update: ${message} (${type})`);
    if (statusDiv) {
      statusDiv.textContent = message;
      statusDiv.className = `status ${type}`;
      if (type === 'success') {
        setTimeout(() => {
          if (statusDiv.textContent === message) {
            statusDiv.textContent = 'Sẵn sàng';
            statusDiv.className = 'status info';
          }
        }, 5000);
      }
    }
  }

  function disableButtons() {
    if (checkBtn) checkBtn.disabled = true;
    if (downloadBtn) downloadBtn.disabled = true;
  }

  function enableButtons() {
    if (checkBtn) checkBtn.disabled = false;
    if (downloadBtn && questionCountDiv.textContent.includes('Tìm thấy')) downloadBtn.disabled = false;
  }

  window.testPopup = function() {
    console.log('🧪 Testing popup elements:', {
      checkBtn: !!checkBtn,
      downloadBtn: !!downloadBtn,
      statusDiv: !!statusDiv,
      questionCountDiv: !!questionCountDiv,
      isConnected: isConnected,
      currentTab: currentTab
    });
  };
});