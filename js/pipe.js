class Pipe {
  constructor(x, y, intake, top) {
    this.x = x;
    this.y = y;
    this.top = top;
    this.active = false;
  }

  draw() {
    push();

    fill(240, 120, 40);
    if (!this.top && !this.active) {
      fill(200);
    }
    strokeWeight(gridSize / 2);
    stroke(100);

    rect(this.x * gridSize, this.y * gridSize, gridSize * 6, gridSize * 2);
    if (this.top) {
      rect((this.x + 1) * gridSize, (this.y - 3) * gridSize, gridSize * 4, gridSize * 3);
    } else {
      rect((this.x + 1) * gridSize, (this.y + 2) * gridSize, gridSize * 4, gridSize * 3);
    }

    pop();
  }
}
