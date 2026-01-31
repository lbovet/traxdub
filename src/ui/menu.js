
function showMenu(options) {
    let selected = 0;
    let menuOptions = options;
    let menuDiv = document.createElement('div');
    menuDiv.className = 'menu-container';
    menuDiv.style.position = 'fixed';
    menuDiv.style.top = '50%';
    menuDiv.style.display = 'flex';
    menuDiv.style.flexDirection = 'column';
    menuDiv.style.alignItems = 'center';
    menuDiv.style.justifyContent = 'center';
    menuDiv.style.userSelect = 'none';
    menuDiv.style.background = 'none';
    menuDiv.style.border = 'none';
    menuDiv.style.outline = 'none';
    menuDiv.style.marginTop = `${Math.max(0, (6 - options.length) * 1.2)}em`;
    menuDiv.style.zIndex = 10;
    menuDiv.style.opacity = 0;
    menuDiv.style.transition = 'opacity 1s';
    setTimeout(() => {
        menuDiv.style.opacity = 1;
    }, 100);

    // Option elements
    let optionDivs = [];
    function render() {
        menuDiv.innerHTML = '';
        const maxVisible = 10;
        const visibleCount = Math.min(maxVisible, menuOptions.length);
        const halfAbove = Math.floor((visibleCount - 1) / 2);
        const halfBelow = Math.floor(visibleCount / 2);
        for (let i = -halfAbove; i <= halfBelow; i++) {
            let idx = (selected + i + menuOptions.length) % menuOptions.length;
            let opt = menuOptions[idx];
            let div = document.createElement('div');
            div.className = 'menu-option';
            div.textContent = opt.label;
            if (i === 0) div.classList.add('selected');
            menuDiv.appendChild(div);
            optionDivs[i + halfAbove] = div;
        }
    }
    render();

    function moveUp() {
        selected = (selected - 1 + menuOptions.length) % menuOptions.length;
        render();
    }
    function moveDown() {
        selected = (selected + 1) % menuOptions.length;
        render();
    }
    function getSelected() {
        return menuOptions[selected];
    }
    document.body.appendChild(menuDiv);

    // Stack option method
    function stackOption() {
        const selectedDiv = optionDivs.find(div => div.classList.contains('selected'));
        if (!selectedDiv) return;
        // Create or get the optionStack div
        let optionStack = document.getElementById('optionStack');
        if (!optionStack) {
            optionStack = document.createElement('div');
            optionStack.id = 'optionStack';
            optionStack.style.position = 'fixed';
            optionStack.style.left = '20px';
            optionStack.style.top = '50%';
            optionStack.style.zIndex = 20;
            optionStack.style.display = 'flex';
            optionStack.style.flexDirection = 'column';
            optionStack.style.alignItems = 'flex-start';
            optionStack.style.marginTop = (5 * 1.2) + 'em';
            document.body.appendChild(optionStack);
        }
        // Decrease top margin by 1.2em, not below zero
        let currentMargin = parseFloat(optionStack.style.marginTop) || 0;
        let newMargin = Math.max(0, currentMargin - 1.2);
        optionStack.style.marginTop = newMargin + 'em';
        // Animate selected option to left
        const rect = selectedDiv.getBoundingClientRect();
        const startX = rect.left;
        const startY = rect.top;
        const endX = 30 + optionStack.children.length * 10; // px from left
        // Clone for animation
        const animDiv = selectedDiv.cloneNode(true);
        animDiv.style.position = 'fixed';
        animDiv.style.left = startX + 'px';
        animDiv.style.top = startY + 'px';
        animDiv.style.margin = '0';
        animDiv.style.transition = 'left 150ms, color 150ms';
        animDiv.style.zIndex = 30;
        document.body.appendChild(animDiv);
        // Fade out other options
        optionDivs.forEach(div => {
            if (div !== selectedDiv) {
                div.style.transition = 'opacity 150ms';
                div.style.opacity = '0';
            }
        });
        // Hide original selected
        selectedDiv.style.opacity = '0';
        // Animate
        setTimeout(() => {
            animDiv.style.left = endX + 'px';
        }, 10);
        // After animation, move to stack and cleanup
        setTimeout(() => {
            animDiv.style.position = '';
            animDiv.style.left = '';
            animDiv.style.top = '';
            animDiv.style.margin = '';
            animDiv.style.transition = '';
            // Store original position for unstacking
            animDiv.dataset.originalX = startX;
            animDiv.dataset.originalY = startY;
            // Indent each new item by 10px more than the previous
            const stackCount = optionStack.children.length;
            animDiv.style.marginLeft = (10 * (stackCount + 1)) + 'px';
            optionStack.appendChild(animDiv);
            if (menuDiv.parentNode) menuDiv.parentNode.removeChild(menuDiv);
        }, 170);
    }

    // Unstack option method - reverses stackOption
    function unstackOption() {
        const optionStack = document.getElementById('optionStack');
        if (!optionStack || optionStack.children.length === 0) return;

        // Get the last stacked item
        const lastStacked = optionStack.lastElementChild;
        const stackedText = lastStacked.textContent;

        // Find the option that was stacked
        const stackedOption = menuOptions.find(opt => opt.label === stackedText);
        if (!stackedOption) return;

        // Get the position of the stacked element before animation
        const rect = lastStacked.getBoundingClientRect();
        const startX = rect.left;
        const startY = rect.top;

        // Remove from stack
        optionStack.removeChild(lastStacked);

        // Increase top margin by 1.2em
        let currentMargin = parseFloat(optionStack.style.marginTop) || 0;
        optionStack.style.marginTop = (currentMargin + 1.2) + 'em';

        // If stack is now empty, remove it
        if (optionStack.children.length === 0) {
            optionStack.parentNode.removeChild(optionStack);
        }

        // Get original position from stored data
        const originalX = parseFloat(lastStacked.dataset.originalX);
        const originalY = parseFloat(lastStacked.dataset.originalY);

        // Position the item for animation
        const animDiv = lastStacked.cloneNode(true);
        animDiv.style.position = 'fixed';
        animDiv.style.left = startX + 'px';
        animDiv.style.top = originalY + 'px';
        animDiv.style.margin = '0';
        animDiv.style.marginLeft = '0';
        animDiv.style.transition = 'left 150ms, top 150ms, opacity 300ms';
        animDiv.style.zIndex = 30;
        document.body.appendChild(animDiv);

        // Animate back to original position
        setTimeout(() => {
            animDiv.style.left = originalX + 'px';
            animDiv.style.top = originalY + 'px';
        }, 10);

        // After animation, restore menu
        setTimeout(() => {
            // Set selected to the unstacked option
            selected = menuOptions.findIndex(opt => opt.label === stackedText);
            if (selected === -1) selected = 0;

            // Show menu again
            menuDiv.style.transition = 'opacity 300ms';
            menuDiv.style.display = 'flex';
            menuDiv.style.opacity = 0;
            document.body.appendChild(menuDiv);
            render();

            setTimeout(() => {
                menuDiv.style.opacity = 1;
                animDiv.style.opacity = 0;
            }, 20);
            setTimeout(() => {
                // Remove the animated div after animation
                if (animDiv.parentNode) {
                    animDiv.parentNode.removeChild(animDiv);
                }
            }, 500);
        }, 120);

    }

    return { moveUp, moveDown, getSelected, menuDiv, stackOption, unstackOption };
}
