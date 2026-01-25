function showMenu(options) {    
    // Fade out and remove any existing menu
    const existingMenu = document.getElementsByClassName('menu-container')[0];
    if (existingMenu) {
        existingMenu.style.transition = 'opacity 0.3s';
        existingMenu.style.opacity = '0';
        setTimeout(() => {
            if (existingMenu.parentNode) {
                existingMenu.parentNode.removeChild(existingMenu);
            }
        }, 300);
    }
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
    return { moveUp, moveDown, getSelected, menuDiv };
}
