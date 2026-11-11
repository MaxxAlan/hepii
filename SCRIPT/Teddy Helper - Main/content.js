/**
 * Khởi tạo extension
 */
console.log('🔌 Content script loaded on:', window.location.href);

// Biến global để theo dõi trạng thái
let isExtensionReady = false;
let connectionRetries = 0;
const MAX_RETRIES = 5;

// Kiểm tra xem trang đã load xong chưa
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initializeExtensionWithRetry);
} else {
  setTimeout(initializeExtensionWithRetry, 100);
}

// Hàm kiểm tra kết nối với trang
function checkPageConnection() {
  try {
    const requiredElements = ['body', 'html'];
    
    for (const selector of requiredElements) {
      if (!document.querySelector(selector)) {
        throw new Error(`Không tìm thấy element: ${selector}`);
      }
    }
    
    if (document.readyState !== 'complete' && document.readyState !== 'interactive') {
      throw new Error('DOM chưa sẵn sàng');
    }
    
    console.log('✅ Kết nối trang thành công');
    return true;
    
  } catch (error) {
    console.warn('⚠️ Lỗi kết nối trang:', error.message);
    return false;
  }
}

// Hàm khởi tạo với retry
async function initializeExtensionWithRetry() {
  console.log(`🔄 Thử kết nối lần ${connectionRetries + 1}/${MAX_RETRIES}`);
  
  if (checkPageConnection()) {
    isExtensionReady = true;
    await initializeExtension();
    sendStatusToPopup('ready', 'Extension đã sẵn sàng');
    return;
  }
  
  connectionRetries++;
  
  if (connectionRetries < MAX_RETRIES) {
    setTimeout(initializeExtensionWithRetry, 1000 + (connectionRetries * 500));
  } else {
    console.error('❌ Không thể kết nối sau', MAX_RETRIES, 'lần thử');
    sendStatusToPopup('error', 'Lỗi kết nối với trang. Vui lòng refresh trang và thử lại.');
  }
}

// Hàm gửi trạng thái đến popup
function sendStatusToPopup(status, message) {
  try {
    chrome.runtime.sendMessage({
      type: 'STATUS_UPDATE',
      status: status,
      message: message
    }).catch(err => {
      console.warn('Không thể gửi message đến popup:', err);
    });
  } catch (error) {
    console.warn('Lỗi khi gửi status:', error);
  }
}

// Hàm khởi tạo extension
function initializeExtension() {
  console.log('🚀 Extension initialized for Quiz page');
  
  // Test connection
  try {
    chrome.runtime.sendMessage({action: 'test'}, (response) => {
      if (chrome.runtime.lastError) {
        console.error('❌ Runtime error:', chrome.runtime.lastError);
      } else {
        console.log('✅ Extension connected successfully');
      }
    });
  } catch (error) {
    console.error('❌ Connection test failed:', error);
  }
}

// Hàm kiểm tra số lượng câu hỏi
async function checkQuestionCount() {
  console.log('🔍 Đang tìm kiếm câu hỏi...');
  
  const questionItems = findQuestionItems();
  
  console.log(`✅ Tìm thấy ${questionItems.length} câu hỏi (ảnh hoặc text)`);
  return questionItems.length;
}

// Hàm tìm câu hỏi (ảnh hoặc text)
function findQuestionItems() {
  console.log('🔍 Đang quét câu hỏi...');
  
  const questionItems = [];
  
  const questionContainers = document.querySelectorAll(
    '.css-1b39eh8, [class*="question"], [class*="item"], [class*="quiz"], [class*="card"], [data-question], [id*="question"], [class*="problem"]'
  );
  
  questionContainers.forEach((container, index) => {
    const questionBadge = container.querySelector('.chakra-badge.css-1dfttbi');
    let questionNumber = null;
    
    if (questionBadge) {
      const badgeText = questionBadge.textContent.trim();
      const match = badgeText.match(/Câu\s*(\d+)/i) || badgeText.match(/(\d+)/);
      if (match) {
        questionNumber = parseInt(match[1]);
      }
    }
    
    if (!questionNumber) {
      questionNumber = index + 1;
    }
    
    // Tìm ảnh
    const images = container.querySelectorAll('img');
    let hasValidImage = false;
    
    images.forEach(img => {
      if (isValidQuestionImage(img)) {
        hasValidImage = true;
        questionItems.push({
          type: 'image',
          img: img,
          questionNumber: questionNumber,
          container: container
        });
        console.log(`📸 Tìm thấy ảnh câu ${questionNumber}: ${img.src.substring(0, 80)}...`);
      }
    });
    
    // Tìm text nếu không có ảnh
    if (!hasValidImage) {
      const questionTextElement = container.querySelector('.css-41t0aq .chakra-text.css-1x53p33');
      const answerElements = container.querySelectorAll('.css-bqomv5 .chakra-text.css-10350gv');
      
      if (questionTextElement && questionTextElement.textContent.trim()) {
        const questionText = questionTextElement.textContent.trim();
        const answers = Array.from(answerElements).map(el => el.textContent.trim()).filter(text => text);
        
        if (answers.length > 0) {
          questionItems.push({
            type: 'text',
            questionText: questionText,
            answers: answers,
            questionNumber: questionNumber,
            container: container
          });
          console.log(`📝 Tìm thấy câu hỏi text ${questionNumber}: ${questionText.substring(0, 80)}...`);
        }
      }
    }
  });
  
  return questionItems;
}

// Hàm kiểm tra ảnh hợp lệ
function isValidQuestionImage(img) {
  const minSize = 50;
  const validFormats = ['jpg', 'jpeg', 'png', 'webp'];
  
  try {
    if (!img.src || !img.complete || img.naturalWidth < minSize || img.naturalHeight < minSize) {
      return false;
    }
    
    const src = img.src.toLowerCase();
    if (src.includes('logo') || src.includes('icon') || src.includes('avatar') || src.includes('banner')) {
      return false;
    }
    
    const extension = getImageExtension(src);
    return validFormats.includes(extension);
  } catch (error) {
    console.warn('⚠️ Lỗi khi kiểm tra ảnh:', error);
    return false;
  }
}

// Hàm chuyển text thành ảnh
// Hàm chuyển text thành ảnh
async function textToImage(questionText, answers, questionNumber) {
  return new Promise((resolve, reject) => {
    try {
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      
      const padding = 30;
      const lineHeight = 45;
      const fontSize = 28;
      const maxWidth = 1100; // Giảm một chút để có thêm margin
      const questionAnswerSpacing = lineHeight * 1.5; // Cách 1.5 dòng
      
      // Thiết lập font trước khi tính toán
      ctx.font = `bold ${fontSize}px Arial, sans-serif`;
      
      // Xử lý câu hỏi - tự động xuống dòng nếu quá dài
      const questionLines = wrapText(ctx, questionText, maxWidth - 2 * padding);
      
      // Thiết lập font cho đáp án
      ctx.font = `${fontSize}px Arial, sans-serif`;
      
      // Xử lý đáp án - mỗi đáp án cũng có thể xuống dòng
      const processedAnswers = [];
      answers.forEach((answer, index) => {
        const optionLetter = String.fromCharCode(65 + index) + '. ';
        const cleanAnswer = answer.replace(/^[A-Za-z]\.\s*/, '');
        const fullAnswer = optionLetter + cleanAnswer;
        const answerLines = wrapText(ctx, fullAnswer, maxWidth - 2 * padding);
        processedAnswers.push(answerLines);
      });
      
      // Tính tổng số dòng
      const totalQuestionLines = questionLines.length;
      const totalAnswerLines = processedAnswers.reduce((total, lines) => total + lines.length, 0);
      
      // Tính chiều cao canvas với padding đủ
      const canvasHeight = padding * 2 + 
                          (totalQuestionLines * lineHeight) + 
                          questionAnswerSpacing + 
                          (totalAnswerLines * lineHeight) + 
                          lineHeight; // Thêm chỗ cho số câu
      
      canvas.width = maxWidth;
      canvas.height = canvasHeight;
      
      // Nền trắng
      ctx.fillStyle = '#FFFFFF';
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      
      // Viền đen
      ctx.strokeStyle = '#000000';
      ctx.lineWidth = 2;
      ctx.strokeRect(1, 1, canvas.width - 2, canvas.height - 2);
      
      // Vẽ câu hỏi
      ctx.fillStyle = '#000000';
      ctx.font = `bold ${fontSize}px Arial, sans-serif`;
      let yOffset = padding;
      
      questionLines.forEach((line, index) => {
        yOffset += lineHeight;
        ctx.fillText(line, padding, yOffset);
      });
      
      // Cách khoảng trước khi vẽ đáp án
      yOffset += questionAnswerSpacing;
      
      // Vẽ các đáp án
      ctx.font = `${fontSize}px Arial, sans-serif`;
      processedAnswers.forEach((answerLines) => {
        answerLines.forEach((line) => {
          yOffset += lineHeight;
          ctx.fillText(line, padding, yOffset);
        });
      });
      
      // Số câu hỏi ở góc phải dưới
      ctx.fillStyle = '#FF0000';
      ctx.font = `bold ${fontSize + 4}px Arial, sans-serif`;
      const numberText = `Câu ${questionNumber}`;
      const textWidth = ctx.measureText(numberText).width;
      const numberX = canvas.width - padding - textWidth;
      const numberY = canvas.height - padding/2;
      ctx.fillText(numberText, numberX, numberY);
      
      canvas.toBlob(blob => {
        if (blob) {
          const reader = new FileReader();
          reader.onload = () => resolve(new Uint8Array(reader.result));
          reader.onerror = () => reject(new Error('Failed to read canvas blob'));
          reader.readAsArrayBuffer(blob);
        } else {
          reject(new Error('Failed to create canvas blob'));
        }
      }, 'image/png', 0.9);
    } catch (error) {
      reject(error);
    }
  });
}

// Hàm bọc văn bản - cải tiến để xử lý tốt hơn
function wrapText(ctx, text, maxWidth) {
  const words = text.split(' ');
  const lines = [];
  let currentLine = '';
  
  for (let i = 0; i < words.length; i++) {
    const word = words[i];
    const testLine = currentLine === '' ? word : currentLine + ' ' + word;
    const testWidth = ctx.measureText(testLine).width;
    
    if (testWidth <= maxWidth || currentLine === '') {
      currentLine = testLine;
    } else {
      // Đẩy dòng hiện tại vào mảng và bắt đầu dòng mới
      if (currentLine !== '') {
        lines.push(currentLine);
      }
      currentLine = word;
    }
  }
  
  // Thêm dòng cuối cùng
  if (currentLine !== '') {
    lines.push(currentLine);
  }
  
  return lines.length > 0 ? lines : [''];
}

// Hàm bọc văn bản
function wrapText(ctx, text, maxWidth) {
  const words = text.split(' ');
  const lines = [];
  let currentLine = words[0] || '';
  
  for (let i = 1; i < words.length; i++) {
    const word = words[i];
    const width = ctx.measureText(currentLine + ' ' + word).width;
    if (width < maxWidth) {
      currentLine += ' ' + word;
    } else {
      lines.push(currentLine);
      currentLine = word;
    }
  }
  lines.push(currentLine);
  return lines;
}

// Hàm tải toàn bộ câu hỏi
async function downloadAllQuestionImages() {
  console.log('📥 Bắt đầu tải tất cả câu hỏi...');
  
  const questionItems = findQuestionItems();
  const zip = new JSZip();
  let successCount = 0;
  
  for (const item of questionItems) {
    try {
      let imageData;
      const fileName = `Cau_${item.questionNumber}.png`;
      
      if (item.type === 'image') {
        // Thêm số câu vào ảnh gốc
        imageData = await addNumberToImage(item.img, item.questionNumber);
      } else if (item.type === 'text') {
        imageData = await textToImage(item.questionText, item.answers, item.questionNumber);
      }
      
      zip.file(fileName, imageData);
      successCount++;
    } catch (error) {
      console.error(`❌ Lỗi khi xử lý câu ${item.questionNumber}:`, error);
    }
  }
  
  if (successCount === 0) {
    throw new Error('Không tải được câu hỏi nào');
  }
  
  const zipBlob = await zip.generateAsync({ type: 'blob' });
  await downloadZipFile(zipBlob, `PinkTeddy_Quiz_${new Date().toISOString().slice(0, 19).replace(/:/g, '-')}.zip`);
  
  return {
    success: true,
    message: `Đã tải ${successCount} câu hỏi`,
    successCount,
    totalItems: questionItems.length
  };
}

// Hàm thêm số câu vào ảnh
async function addNumberToImage(img, questionNumber) {
  return new Promise((resolve, reject) => {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    
    // Thiết lập kích thước canvas bằng kích thước ảnh gốc
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    
    // Vẽ ảnh gốc lên canvas
    ctx.drawImage(img, 0, 0);
    
    // Thêm số câu vào góc phải dưới (giống textToImage)
    const padding = 20;
    const fontSize = 20;
    ctx.fillStyle = '#FF0000';
    ctx.font = `bold ${fontSize + 2}px Arial, sans-serif`;
    const numberText = `Câu ${questionNumber}`;
    const textWidth = ctx.measureText(numberText).width;
    ctx.fillText(numberText, canvas.width - padding - textWidth, canvas.height - padding);
    
    canvas.toBlob(blob => {
      if (blob) {
        const reader = new FileReader();
        reader.onload = () => resolve(new Uint8Array(reader.result));
        reader.onerror = () => reject(new Error('Failed to read image blob'));
        reader.readAsArrayBuffer(blob);
      } else {
        reject(new Error('Failed to create image blob'));
      }
    }, 'image/png');
  });
}

// Hàm lấy dữ liệu ảnh
async function getImageData(img) {
  return new Promise((resolve, reject) => {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    ctx.drawImage(img, 0, 0);
    
    canvas.toBlob(blob => {
      if (blob) {
        const reader = new FileReader();
        reader.onload = () => resolve(new Uint8Array(reader.result));
        reader.onerror = () => reject(new Error('Failed to read image blob'));
        reader.readAsArrayBuffer(blob);
      } else {
        reject(new Error('Failed to create image blob'));
      }
    }, 'image/png');
  });
}

// Message listener
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  console.log('📨 Received message:', request.action);
  
  if (!isExtensionReady && request.action !== 'test' && request.action !== 'CHECK_STATUS') {
    sendResponse({ success: false, error: 'Extension chưa sẵn sàng' });
    return;
  }
  
  try {
    switch (request.action) {
      case 'test':
        sendResponse({ success: true, message: 'Connection OK' });
        break;
      case 'CHECK_STATUS':
        sendResponse({ success: true, ready: isExtensionReady, message: isExtensionReady ? 'Sẵn sàng' : 'Đang kết nối...' });
        break;
      case 'checkQuestions':
        checkQuestionCount().then(count => sendResponse({ success: true, totalQuestions: count }))
          .catch(error => sendResponse({ success: false, error: error.message }));
        return true;
      case 'downloadAllImages':
        downloadAllQuestionImages().then(result => sendResponse(result))
          .catch(error => sendResponse({ success: false, error: error.message }));
        return true;
    }
  } catch (error) {
    console.error('❌ Lỗi xử lý message:', error);
    sendResponse({ success: false, error: error.message });
  }
  return true;
});

// Tải file ZIP
async function downloadZipFile(zipBlob, fileName) {
  const url = URL.createObjectURL(zipBlob);
  const a = document.createElement('a');
  a.href = url;
  a.download = fileName;
  a.style.display = 'none';
  
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

// Lấy MIME type và extension
function getImageExtension(url) {
  if (url.startsWith('data:image/')) {
    const mimeMatch = url.match(/data:image\/([^;]+)/);
    if (mimeMatch) return mimeMatch[1] === 'jpeg' ? 'jpg' : mimeMatch[1];
    return 'png';
  }
  try {
    const pathname = new URL(url).pathname;
    const extension = pathname.split('.').pop().toLowerCase();
    const validExtensions = ['jpg', 'jpeg', 'png', 'gif', 'webp'];
    return validExtensions.includes(extension) ? extension : 'png';
  } catch {
    return 'png';
  }
}