/**
 * Creates an SVG rotary knob element with value display and interactive controls
 * @param {string} color - The color to use for all elements
 * @param {number} value - Initial rotary value (0 to 1)
 * @returns {Object} Object with element, setValue, and setColor functions
 */
function createRotary(color, value) {
  // Constants
  const SIZE = 100;
  const CENTER = SIZE / 2;
  const MAIN_RADIUS = 22;
  const MAIN_WIDTH = 1;
  const SUPPORT_RADIUS = MAIN_RADIUS - MAIN_WIDTH / 2;
  const START_ANGLE = 180; // 30 degrees left of bottom (240 degrees is bottom + 150)
  const ARC_SPAN = 359.9; // Total arc span in degrees
  const TICK_POSITIONS = [0, ARC_SPAN]; // Degrees along the arc

  // Create SVG element
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('width', SIZE);
  svg.setAttribute('height', SIZE);
  svg.setAttribute('viewBox', `0 0 ${SIZE} ${SIZE}`);

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
    const start = polarToCartesian(x, y, radius, endAngle);
    const end = polarToCartesian(x, y, radius, startAngle);
    const largeArcFlag = endAngle - startAngle <= 180 ? '0' : '1';
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

  // Create support arc (fixed background arc)
  const supportArc = document.createElementNS('http://www.w3.org/2000/svg', 'path');
  const supportArcPath = describeArc(CENTER, CENTER, SUPPORT_RADIUS, START_ANGLE, START_ANGLE + ARC_SPAN);
  supportArc.setAttribute('d', supportArcPath);
  supportArc.setAttribute('fill', 'none');
  supportArc.setAttribute('stroke', color);
  supportArc.setAttribute('stroke-width', '1');
  supportArc.setAttribute('stroke-linecap', 'butt');
  supportArc.setAttribute('opacity', '1');
  //svg.appendChild(supportArc);

  // Create ticks
  const ticks = [];
  TICK_POSITIONS.forEach((degreeOffset, index) => {
    const tickAngle = START_ANGLE + degreeOffset;
    // All extreme ticks are MAIN_WIDTH long
    const tickLen = MAIN_WIDTH;
    const innerPoint = polarToCartesian(CENTER, CENTER, SUPPORT_RADIUS, tickAngle);
    const outerPoint = polarToCartesian(CENTER, CENTER, SUPPORT_RADIUS + tickLen, tickAngle);

    const tick = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    tick.setAttribute('x1', innerPoint.x);
    tick.setAttribute('y1', innerPoint.y);
    tick.setAttribute('x2', outerPoint.x);
    tick.setAttribute('y2', outerPoint.y);
    tick.setAttribute('stroke', color);
    tick.setAttribute('stroke-width', '1');
    tick.setAttribute('stroke-linecap', 'butt');

    ticks.push(tick);
    //svg.appendChild(tick);
  });

  // Create value text
  const valueText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
  valueText.setAttribute('x', CENTER-0.5);
  valueText.setAttribute('y', CENTER+1);
  valueText.setAttribute('text-anchor', 'middle');
  valueText.setAttribute('dominant-baseline', 'middle');
  valueText.setAttribute('fill', color);
  valueText.setAttribute('font-size', '20');
  svg.appendChild(valueText);

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

    // Update value text (multiply by 100 and round)
    const displayValue = Math.round(clampedValue * 100);
    valueText.textContent = displayValue;
  }

  /**
   * Update all colors
   */
  function setColor(newColor) {
    mainArc.setAttribute('stroke', newColor);
    supportArc.setAttribute('stroke', newColor);
    ticks.forEach(tick => tick.setAttribute('stroke', newColor));
    valueText.setAttribute('fill', newColor);
  }

  // Set initial value
  setValue(value);

  return {
    element: svg,
    setValue: setValue,
    setColor: setColor
  };
}
