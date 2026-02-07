/**
 * Creates an SVG rotary knob element with value display and interactive controls
 * @param {string} color - The color to use for all elements
 * @param {number} value - Initial rotary value (0 to 1)
 * @returns {Object} Object with element, setValue, and setColor functions
 */
function createRotary(color, value) {
  // Track current color
  let currentColor = color;

  // Constants
  const BASE_SIZE = 16;
  const MAIN_WIDTH = 1;
  const ELEMENT_SIZE = BASE_SIZE * 2 + MAIN_WIDTH;
  const CENTER = ELEMENT_SIZE / 2;
  const MAIN_RADIUS = BASE_SIZE - 2;
  const SMALL_RADIUS = 4;
  const START_ANGLE = 180;
  const ARC_SPAN = 360;

  // Create SVG element
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('width', ELEMENT_SIZE);
  svg.setAttribute('height', ELEMENT_SIZE);
  svg.setAttribute('viewBox', `0 0 ${ELEMENT_SIZE} ${ELEMENT_SIZE}`);

  /**
   * Convert polar coordinates to cartesian
   */
  function polarToCartesian(centerX, centerY, radius, angleDeg) {
    const angleRad = (angleDeg - 90) * Math.PI / 180.0;
    return {
      x: centerX + radius * Math.cos(angleRad),
      y: centerY + radius * Math.sin(angleRad)
    };
  }

  /**
   * Create SVG arc path
   */
  function describeArc(x, y, radius, startAngle, endAngle) {
    const angleSpan = endAngle - startAngle;

    // If the arc is a full circle (or very close), split into two semicircles
    if (angleSpan >= 359.9) {
      const midAngle = startAngle + 180;
      const start = polarToCartesian(x, y, radius, startAngle);
      const mid = polarToCartesian(x, y, radius, midAngle);
      const end = polarToCartesian(x, y, radius, endAngle);

      return [
        'M', start.x, start.y,
        'A', radius, radius, 0, 0, 1, mid.x, mid.y,
        'A', radius, radius, 0, 0, 1, end.x, end.y
      ].join(' ');
    }

    const start = polarToCartesian(x, y, radius, endAngle);
    const end = polarToCartesian(x, y, radius, startAngle);
    const largeArcFlag = angleSpan <= 180 ? '0' : '1';
    return [
      'M', start.x, start.y,
      'A', radius, radius, 0, largeArcFlag, 0, end.x, end.y
    ].join(' ');
  }

  // Create main arc (value indicator)
  const mainArc = document.createElementNS('http://www.w3.org/2000/svg', 'path');
  mainArc.setAttribute('fill', 'none');
  mainArc.setAttribute('stroke', color);
  mainArc.setAttribute('stroke-width', MAIN_WIDTH);
  mainArc.setAttribute('stroke-linecap', 'butt');
  svg.appendChild(mainArc);

  // Create value text
  const valueText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
  valueText.setAttribute('x', CENTER - 0.5);
  valueText.setAttribute('y', CENTER + 2);
  valueText.setAttribute('text-anchor', 'middle');
  valueText.setAttribute('dominant-baseline', 'middle');
  valueText.setAttribute('fill', color);
  valueText.setAttribute('font-size', BASE_SIZE);
  svg.appendChild(valueText);

  const smallCircle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
  smallCircle.setAttribute('cx', CENTER);
  smallCircle.setAttribute('cy', CENTER);
  smallCircle.setAttribute('fill', 'none');
  smallCircle.setAttribute('stroke-width', '1');
  svg.appendChild(smallCircle);

  /**
   * Update the rotary value display
   */
  function setValue(newValue) {
    // Clamp value between 0 and 1
    const clampedValue = Math.max(0, Math.min(1, newValue));

    // Calculate end angle based on value
    const endAngle = START_ANGLE + (clampedValue * ARC_SPAN);

    // Update main arc
    if (clampedValue > 0) {
      const mainArcPath = describeArc(CENTER, CENTER, MAIN_RADIUS, START_ANGLE, endAngle);
      mainArc.setAttribute('d', mainArcPath);
      mainArc.style.display = '';
    } else {
      mainArc.style.display = 'none';
    }

    const displayValue = Math.round(clampedValue * 100);

    // Update value display
    if (displayValue === 0) {
      // Show circle (unfilled), hide text
      smallCircle.style.display = '';
      smallCircle.setAttribute('fill', 'none');
      smallCircle.setAttribute('r', SMALL_RADIUS - 1);
      smallCircle.setAttribute('stroke', currentColor);
      valueText.style.display = 'none';
    } else if (displayValue === 100) {
      // Show circle (filled), hide text
      smallCircle.style.display = '';
      smallCircle.setAttribute('r', SMALL_RADIUS);
      smallCircle.setAttribute('fill', currentColor);
      // Hide the circle stroke
      smallCircle.setAttribute('stroke', '#1a1a1a');
      valueText.style.display = 'none';
    } else {
      // Show text, hide circle
      smallCircle.style.display = 'none';
      valueText.style.display = '';
      valueText.textContent = displayValue;
    }
  }

  /**
   * Update all colors
   */
  function setColor(newColor) {
    currentColor = newColor;
    mainArc.setAttribute('stroke', newColor);
    supportArc.setAttribute('stroke', newColor);
    ticks.forEach(tick => tick.setAttribute('stroke', newColor));
    valueText.setAttribute('fill', newColor);
    smallCircle.setAttribute('stroke', newColor);
    // Update fill if circle is currently visible and filled
    if (smallCircle.style.display !== 'none' && smallCircle.getAttribute('fill') !== 'none') {
      smallCircle.setAttribute('fill', newColor);
    }
  }

  // Set initial value
  setValue(value);

  return {
    element: svg,
    setValue: setValue,
    setColor: setColor
  };
}
