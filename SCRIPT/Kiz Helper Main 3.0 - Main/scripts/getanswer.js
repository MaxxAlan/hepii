(async function() {
    // Function to delay execution for a certain amount of time
    const delay = ms => new Promise(resolve => setTimeout(resolve, ms));

    // Select all page buttons
    const pageButtons = document.querySelectorAll('#NC_pag .page-link');
    let result = '';

    for (let i = 0; i < pageButtons.length; i++) {
        const button = pageButtons[i];
        button.click(); // Simulate click to go to the page
        await delay(50); // Wait for the page to load, adjust the delay as needed

        // Select the answer list in the current page
        const answerList = document.querySelector('#NC_Answer');
        if (answerList) {
            // Select all active answers
            const activeAnswers = answerList.querySelectorAll('.page-item.active .page-link');
            if (activeAnswers.length > 0) {
                // Concatenate all active answer texts (A, B, C, etc.)
                const answers = Array.from(activeAnswers)
                    .map(answer => answer.innerText.trim().toLowerCase())
                    .join('');
                
                // Append the question number and answers to the result
                result += `${i + 1}${answers} `;
            }
        }
    }

    // Create and style the popup
    const popup = document.createElement('div');
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
    `;

    // Create the content
    popup.innerHTML = `
        <h3 style="margin-top: 0; margin-bottom: 15px;">Đáp án</h3>
        <textarea style="width: 100%; min-height: 100px; margin-bottom: 10px; padding: 8px;">${result.trim()}</textarea>
        <div style="display: flex; justify-content: space-between;">
            <button id="copyAnswer" style="padding: 8px 15px; cursor: pointer;">Copy đáp án</button>
            <button id="closePopup" style="padding: 8px 15px; cursor: pointer;">Đóng</button>
        </div>
    `;

    // Add the popup to the page
    document.body.appendChild(popup);

    // Add event listeners
    document.getElementById('copyAnswer').addEventListener('click', () => {
        const textarea = popup.querySelector('textarea');
        textarea.select();
        document.execCommand('copy');
        alert('Đã copy đáp án!');
    });

    document.getElementById('closePopup').addEventListener('click', () => {
        popup.remove();
    });
})();
