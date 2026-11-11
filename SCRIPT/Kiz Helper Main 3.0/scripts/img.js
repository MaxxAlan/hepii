(async function () {
    // Lấy nội dung của thẻ title và lọc mã sau "Kizspy - "
    const title = document.title;
    const code = title.split('Kizspy - ')[1]; // Lấy phần mã sau "Kizspy - "

    // Tạo đối tượng JSZip để nén các ảnh đã tải
    const zip = new JSZip();

    // Hàm tải ảnh và thêm số câu hỏi cùng đáp án
    async function downloadImageWithAnswer(url, answers, selectedAnswers, filename, questionNumber) {
        try {
            const response = await fetch(url);
            const blob = await response.blob();
            const imgBitmap = await createImageBitmap(blob);

            // Tạo canvas với kích thước phù hợp
            const canvas = document.createElement('canvas');
            canvas.width = imgBitmap.width;
            canvas.height = imgBitmap.height + 40 * answers.length + 100; // Thêm không gian cho câu hỏi và đáp án

            const ctx = canvas.getContext('2d');

            // Đặt nền màu đen
            ctx.fillStyle = '#000000';
            ctx.fillRect(0, 0, canvas.width, canvas.height);

            // Vẽ số câu hỏi ở trên cùng
            ctx.fillStyle = '#FFFFFF'; // Màu chữ trắng
            ctx.font = 'bold 24px Arial'; // Phông chữ đậm và lớn hơn
            ctx.textAlign = 'left';
            ctx.fillText(`Question ${questionNumber}: ${selectedAnswers}`, 10, 30); // Ghi "Question X: answers"

            // Vẽ ảnh lên canvas (dưới số câu)
            ctx.drawImage(imgBitmap, 0, 50);

            // Vẽ từng đáp án bên dưới ảnh
            ctx.fillStyle = '#FFFFFF'; // Màu chữ trắng
            ctx.font = '18px Arial'; // Kích thước phông chữ đáp án
            answers.forEach((line, index) => {
                ctx.fillText(`${String.fromCharCode(65 + index)}. ${line.trim()}`, 10, imgBitmap.height + 70 + index * 40); // Thêm A, B, C, D
            });

            // Chuyển canvas thành Blob (ảnh đã kết hợp)
            const modifiedBlob = await new Promise(resolve => canvas.toBlob(resolve));

            // Thêm file đã chỉnh sửa vào zip
            zip.file(filename, modifiedBlob);
        } catch (error) {
            console.error(`Error processing image ${filename}:`, error);
        }
    }

    // Tìm tất cả các button trên web
    const buttons = document.querySelectorAll('.page-link.cn-go-to');

    // Số lượng ảnh đã tải
    let imageCount = 0;

    for (let i = 0; i < buttons.length; i++) {
        // Nhấn vào button để tải phần tương ứng
        buttons[i].click();

        // Đợi một khoảng thời gian để trang tải ảnh
        await new Promise(resolve => setTimeout(resolve, 100)); // Tăng lên 100ms để đồng bộ với code thứ nhất

        // Lấy nội dung số câu từ phần tử #NC_num
        const questionElement = document.querySelector('#NC_num');
        const questionNumber = questionElement ? questionElement.textContent.trim().replace('Question ', '') : i + 1;

        // Lấy đáp án chi tiết từ #NC_text p
        const answerElements = document.querySelectorAll('#NC_text p');
        const answers = Array.from(answerElements).map(el => el.textContent.trim());

        // Lấy đáp án được chọn từ #NC_Answer
        const answerList = document.querySelector('#NC_Answer');
        let selectedAnswers = '';
        if (answerList) {
            const activeAnswers = answerList.querySelectorAll('.page-item.active .page-link');
            if (activeAnswers.length > 0) {
                selectedAnswers = Array.from(activeAnswers)
                    .map(answer => answer.innerText.trim().toLowerCase())
                    .join('');
            }
        }

        // Lấy ảnh
        const img = document.querySelector('#NC_img');
        if (img) {
            const src = img.src;
            const filename = `image_${questionNumber}.jpg`;

            // Tải ảnh và thêm số câu hỏi, đáp án được chọn cùng danh sách đáp án
            await downloadImageWithAnswer(src, answers, selectedAnswers, filename, questionNumber);
            imageCount++;
        }
    }

    // Nén và lưu file ZIP với tên dựa trên mã sau "Kizspy - "
    zip.generateAsync({ type: "blob" }).then(function (content) {
        const a = document.createElement('a');
        a.href = URL.createObjectURL(content);
        a.download = `${code}.zip`; // Đặt tên file zip theo mã
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(a.href);
    }).catch(function (err) {
        console.error('Error creating ZIP file:', err);
    });

    console.log(`Đã tải ${imageCount} ảnh và lưu vào file ZIP có tên ${code}.zip.`);
})();