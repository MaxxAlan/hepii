(async function() {
    // Kiểm tra cả hai thư viện
    if (typeof html2canvas === 'undefined' || typeof JSZip === 'undefined') {
        console.error('Required libraries not loaded!');
        alert('Vui lòng nhúng thư viện trước (bước 1)');
        return;
    }

    // Thêm thông báo console để debug
    console.log('Starting text capture process...');

    // 1) Lấy code (tên file ZIP) từ tiêu đề
    const title = document.title || '';
    const code = (title.includes('Kizspy - '))
        ? title.split('Kizspy - ')[1]
        : 'mycapture';

    // 2) Tạo đối tượng JSZip
    const zip = new JSZip();

    // 3) Xác định selector cho phần nội dung cần chụp
    const questionSelector = '#NC_text';  // Vùng cần chụp

    // 4) Lấy danh sách nút phân trang từ #NC_pag
    const paginationButtons = Array.from(document.querySelectorAll('#NC_pag .cn-go-to'));
    if (paginationButtons.length === 0) {
        console.log('Không tìm thấy nút phân trang. Dừng...');
        return;
    }

    // 5) Lặp qua từng nút để chụp
    for (let i = 0; i < paginationButtons.length; i++) {
        // a) Click nút để chuyển sang câu
        paginationButtons[i].click();
        await new Promise(resolve => setTimeout(resolve, 50)); // chờ nội dung load

        // b) Lấy nhãn câu hỏi từ #NC_num (nếu có)
        const ncNumEl = document.querySelector('#NC_num');
        const questionLabel = ncNumEl ? ncNumEl.textContent.trim() : `Question ${i+1}`;

        // c) Tìm vùng cần chụp
        const el = document.querySelector(questionSelector);
        if (!el) {
            console.log(`Không tìm thấy vùng cho câu ${i+1}. Bỏ qua...`);
            continue;
        }

        try {
            // d) Sử dụng html2canvas với onclone để thêm overlay label
            const canvas = await html2canvas(el, {
                useCORS: true,
                onclone: (clonedDoc) => {
                    const clonedEl = clonedDoc.querySelector(questionSelector);
                    if (clonedEl) {
                        // Tạo overlay chứa label
                        const overlay = clonedDoc.createElement('div');
                        overlay.innerHTML = questionLabel;
                        // Thiết lập style cho overlay
                        overlay.style.position = 'absolute';
                        overlay.style.top = '10px';
                        overlay.style.right = '10px';
                        overlay.style.fontSize = '15px';
                        overlay.style.fontWeight = 'bold';
                        overlay.style.color = 'red';
                        overlay.style.zIndex = '9999';
                        // Đảm bảo overlay không bị tràn ra ngoài vùng cần chụp
                        clonedEl.style.position = 'relative';
                        clonedEl.appendChild(overlay);
                    }
                }
            });

            // e) Chuyển canvas sang blob để thêm vào ZIP
            const dataURL = canvas.toDataURL('image/png');
            const response = await fetch(dataURL);
            const blob = await response.blob();
            zip.file(`question_${i+1}.png`, blob);

            console.log(`Đã chụp và đánh dấu: ${questionLabel}`);
        } catch (err) {
            console.error(`Lỗi chụp câu ${i+1}:`, err);
        }
    }

    // 6) Sau khi chụp xong, tạo file ZIP và tải về
    try {
        const content = await zip.generateAsync({ type: 'blob' });
        const a = document.createElement('a');
        a.href = URL.createObjectURL(content);
        a.download = `${code}.zip`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(a.href);
        console.log(`Đã nén và tải file ZIP: ${code}.zip`);
    } catch (err) {
        console.error('Lỗi tạo ZIP:', err);
    }
})();
