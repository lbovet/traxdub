/**
 * Creates a control element with a rotary knob and label
 * @param {string} color - The color to use for the rotary
 * @param {number} value - Initial rotary value (0 to 1)
 * @param {string} name - The name label for the control
 * @param {string} title - Optional title above the name (can be empty string)
 * @returns {Object} Object with element and control functions
 */
function createControl(color, value, name, title) {
  // Create main container
  const container = document.createElement('div');
  container.style.display = 'flex';
  container.style.alignItems = 'center';
  container.style.gap = '4px';

  // Create rotary
  const rotary = createRotary(color, value);
  container.appendChild(rotary.element);

  // Create label container
  const labelContainer = document.createElement('div');
  labelContainer.style.display = 'flex';
  labelContainer.style.flexDirection = 'column';
  labelContainer.style.transform = 'translateY(-8px)';  // Shift up to align name center with rotary center

  // Create title div (always reserve space)
  const titleDiv = document.createElement('div');
  titleDiv.style.height = '1.2em';
  titleDiv.style.lineHeight = '1.2em';
  titleDiv.style.fontSize = '16px';
  titleDiv.style.opacity = '0.5';
  titleDiv.style.color = color;
  titleDiv.style.whiteSpace = 'nowrap';
  titleDiv.textContent = title || '';
  labelContainer.appendChild(titleDiv);

  // Create name div
  const nameDiv = document.createElement('div');
  nameDiv.style.height = '1.2em';
  nameDiv.style.lineHeight = '1.2em';
  nameDiv.style.color = color;
  nameDiv.style.whiteSpace = 'nowrap';
  nameDiv.textContent = name;
  labelContainer.appendChild(nameDiv);

  container.appendChild(labelContainer);

  /**
   * Update the name and title labels
   */
  function setLabel(newName, newTitle) {
    nameDiv.textContent = newName;
    titleDiv.textContent = newTitle || '';
  }

  /**
   * Update the color of all elements
   */
  function setColor(newColor) {
    rotary.setColor(newColor);
    titleDiv.style.color = newColor;
    nameDiv.style.color = newColor;
  }

  return {
    element: container,
    setValue: rotary.setValue,
    setColor: setColor,
    setLabel: setLabel
  };
}

/**
 * Creates a board element with a grid of controls
 * @param {number} rows - Number of rows in the grid
 * @param {number} columns - Number of columns in the grid
 * @returns {Object} Object with element and getControl function
 */
function createBoard(rows, columns) {
  // Create main board container
  const board = document.createElement('div');
  board.style.position = 'fixed';
  board.style.bottom = '2em';
  board.style.left = '50%';
  board.style.transform = 'translateX(-50%)';
  board.style.display = 'grid';
  board.style.gridTemplateColumns = `repeat(${columns}, minmax(0, 1fr))`;
  board.style.gridTemplateRows = `repeat(${rows}, auto)`;
  board.style.gap = '10px';
  // Width is ideal size (10em per column + gaps) but capped at window width - padding
  const idealWidth = `calc(${columns * 10}em + ${(columns - 1) * 10}px)`;
  const maxWidth = 'calc(100vw - 4em)';
  board.style.width = `min(${idealWidth}, ${maxWidth})`;

  // Create cells with controls
  const controls = [];
  for (let row = 0; row < rows; row++) {
    controls[row] = [];
    for (let col = 0; col < columns; col++) {
      // Create cell wrapper
      const cell = document.createElement('div');
      cell.style.height = '2.8em';
      cell.style.overflow = 'hidden';
      cell.style.position = 'relative';
      cell.style.display = 'flex';
      cell.style.alignItems = 'flex-end';
      cell.style.justifyContent = 'flex-start';

      // Add fade effect for overflow
      const fadeOverlay = document.createElement('div');
      fadeOverlay.style.position = 'absolute';
      fadeOverlay.style.top = '0';
      fadeOverlay.style.right = '0';
      fadeOverlay.style.bottom = '0';
      fadeOverlay.style.width = '20px';
      fadeOverlay.style.background = 'linear-gradient(to right, transparent, #1a1a1a)';
      fadeOverlay.style.pointerEvents = 'none';
      fadeOverlay.style.zIndex = '1';
      cell.appendChild(fadeOverlay);

      // Create default control
      const control = createControl('#66ffff', 0, '', '');
      cell.appendChild(control.element);

      controls[row][col] = control;
      board.appendChild(cell);
    }
  }

  /**
   * Get control at specific position
   */
  function getControl(row, col) {
    return controls[row]?.[col];
  }

  return {
    element: board,
    getControl: getControl
  };
}
