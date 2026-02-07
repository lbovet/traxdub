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
  titleDiv.textContent = title || '';
  labelContainer.appendChild(titleDiv);

  // Create name div
  const nameDiv = document.createElement('div');
  nameDiv.style.height = '1.2em';
  nameDiv.style.lineHeight = '1.2em';
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

  return {
    element: container,
    setValue: rotary.setValue,
    setColor: rotary.setColor,
    setLabel: setLabel
  };
}
