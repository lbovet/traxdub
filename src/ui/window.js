let ratio = 1.0;
const size = 2000;
document.getElementById('main').setAttribute('width', size * ratio);
document.getElementById('main').setAttribute('height', size * ratio);

// Global state
// Note: currentMenu is defined in menu.js and accessed here for navigation routing
let grid = null;

function init(prepareCallback, doneCallback) {
    window.addEventListener('load', async function() {
        await loadLogo(ratio, () => {
            const bullet = document.getElementById('bullet');
            bullet.style.transition = 'r 1.2s ease-out';
            bullet.setAttribute('r', 4);
            prepareCallback();
        },
        () => {
            const bullet = document.getElementById('bullet');
            bullet.style.transition = 'stroke-width 0.4s ease-in';
            bullet.setAttribute('stroke-width', 1);
            setTimeout(() => {
                bullet.remove();
            }, 400);
            doneCallback();
        });
        document.body.classList.add('loaded');
        if(window.ipc) {
            window.ipc.postMessage('page-loaded');
        }
    });
}

function updateSize() {
    const svg = document.getElementById('main');
    if (!svg) return;
    const pt = svg.createSVGPoint();
    pt.x = 0;
    pt.y = 0;
    const svgCoords = pt.matrixTransform(svg.getScreenCTM().inverse());

    // Move the graph vertically between the screen top and the center
    const graph = document.getElementById('graph');
    const newY = (size / 2 - Math.round(svgCoords.y)) * 0.5;
    graph.setAttribute('transform', `translate(0, ${-newY})`);
}
window.addEventListener('resize', updateSize);
window.addEventListener('DOMContentLoaded', updateSize);

// ============================================================================
// Message Polling and Handling
// ============================================================================

async function pollMessages() {
    try {
        const response = await fetch('/messages');
        const messages = await response.json();
        
        if (messages.length > 0) {
            console.log(`Processing ${messages.length} messages`);
            
            // Process all messages
            for (const msgStr of messages) {
                try {
                    const message = JSON.parse(msgStr);
                    handleMessage(message);
                } catch (e) {
                    console.error('Failed to parse message:', msgStr, e);
                    sendError('Message parse error', e.message);
                }
            }
        }
    } catch (e) {
        console.error('Failed to poll messages:', e);
    }
    
    // Continue polling
    requestAnimationFrame(pollMessages);
}

function handleMessage(message) {
    console.log('Received message:', message);
    try {
        const { type, data } = message;
        
        switch (type) {
            case 'create_node':
                handleCreateNode(data);
                break;
            case 'create_link':
                handleCreateLink(data);
                break;
            case 'insert_node':
                handleInsertNode(data);
                break;
            case 'navigate':
                handleNavigate(data);
                break;
            case 'open_menu':
                handleOpenMenu(data);
                break;
            case 'close_menu':
                handleCloseMenu();
                break;
            case 'close_all_menus':
                handleCloseAllMenus();
                break;
            case 'commit':
                handleCommit();
                break;
            case 'prompt':
                handlePrompt(data);
                break;
            default:
                console.warn('Unknown message type:', type);
        }
    } catch (e) {
        console.error('Error handling message:', message, e);
        sendError('Message handler error', e.message, message);
    }
}

// ============================================================================
// Grid Handlers
// ============================================================================

function handleCreateNode(data) {
    const { id, label, nodeType } = data;
    
    // Map node types to grid styling
    const boxOptions = { label };
    
    if (nodeType === 'context') {
        delete boxOptions.label; // Context nodes have no label
        boxOptions.invisible = true;
    }
    
    // For now, use hardcoded positioning
    grid.setBox(id, boxOptions, 0, id === 'inputs' ? 0 : 1);
    
    console.log(`Created node: ${id} (${label})`);
}

function handleCreateLink(data) {
    const { fromId, toId, linkType } = data;
    
    grid.addLine(fromId, toId);
    
    // Focus the first link created
    if (fromId === 'inputs' && toId === 'outputs') {
        grid.focusLine(fromId, toId);
    }

    console.log(`Created link: ${fromId} -> ${toId}`);
}

function handleInsertNode(data) {
    const { id, label, nodeType, linkFrom, linkTo } = data;
    
    // Remove old link (unless it's inputs to outputs)
    if (linkFrom !== 'inputs' || linkTo !== 'outputs') {
        grid.removeLine(linkFrom, linkTo);
    }
    
    // Create new node
    const boxOptions = { label };
    grid.setBox(id, boxOptions);
    
    // Create new links
    grid.addLine(linkFrom, id);
    grid.addLine(id, linkTo);
    
    // Focus the new node
    grid.focusBox(id);
    
    console.log(`Inserted node: ${id} between ${linkFrom} and ${linkTo}`);
}

function handleNavigate(data) {
    const { level, direction } = data;
    
    // If menu is open, navigate the menu
    if (currentMenu !== null) {
        if (direction === 'forward') {
            currentMenu.moveDown();
        } else if (direction === 'backward') {
            currentMenu.moveUp();
        }
        return;
    }
    
    // Otherwise navigate the grid
    if (level === 'main') {
        if (direction === 'forward') {
            grid.moveFocusRight();
        } else if (direction === 'backward') {
            grid.moveFocusLeft();
        }
    } else if (level === 'secondary') {
        if (direction === 'forward') {
            grid.moveFocusDown();
        } else if (direction === 'backward') {
            grid.moveFocusUp();
        }
    }
}

function handleCommit() {
    grid.commit();
    console.log('Committed visual changes');
}

// ============================================================================
// Menu Handlers
// ============================================================================

function handleOpenMenu(data) {
    const { id, label, options } = data;
    
    currentMenu = showMenu(options);
    
    console.log(`Opened menu: ${label} with ${options.length} options`);
}

function handleCloseMenu() {
    if (currentMenu) {
        const hadParent = currentMenu.close();
        if (!hadParent) {
            currentMenu = null;
        }
        window.ipc.postMessage(JSON.stringify({
            type: 'menu_closed'
        }));
    }
}

function handleCloseAllMenus() {
    if (currentMenu) {
        currentMenu.exit();
        currentMenu = null;
    }
    window.ipc.postMessage(JSON.stringify({
        type: 'menu_stack_changed',
        data: { size: 0 }
    }));
}

// ============================================================================
// Prompt Handler
// ============================================================================

function handlePrompt(data) {
    const { message } = data;
    const promptArea = document.getElementById('prompt-area');
    if (promptArea) {
        promptArea.textContent = message;
        promptArea.style.display = 'block';
        
        // Auto-hide after 5 seconds
        setTimeout(() => {
            if (promptArea.textContent === message) {
                promptArea.style.display = 'none';
            }
        }, 5000);
    }
}

// ============================================================================
// Error Reporting
// ============================================================================

function sendError(title, message, context) {
    window.ipc.postMessage(JSON.stringify({
        type: 'error',
        data: {
            title,
            message,
            context: context || null
        }
    }));
}

// ============================================================================
// Initialization
// ============================================================================

function startUI() {
    // Initialize grid
    const graphSvg = document.getElementById('main');
    grid = createGrid(graphSvg);
    
    // Start message polling (only in wry WebView)
    if (typeof window.ipc !== 'undefined') {
        pollMessages();
        console.log('UI initialized, polling for messages');
    } else {                
        window.ipc = { postMessage: console.log }; // Fallback to console.log in normal browser
        console.log('Running in normal browser - message polling disabled');
    }
}

init(() => {}, startUI);

// Create control board
const board = createBoard(2, 4);
document.body.appendChild(board.element);

// Configure controls with various examples
// Row 0
board.getControl(0, 0).setLabel('Gain', 'Master');
