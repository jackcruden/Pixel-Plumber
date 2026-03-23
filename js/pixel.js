class Pixel {
  constructor(pos, s, e) {
    this.pos = pos;
    this.s = (s !== undefined) ? s : AIR;
    this.e = (e !== undefined) ? e : 0;
  }

  draw() {
    push();
    noStroke();
    strokeWeight(0.001);

    switch (this.e) {
      case AIR:
        fill(255, 20);
        break;
      case WATER:
        fill(20, 150, 250);
        break;
      case ACID:
        fill(140, 180, 100);
        break;
      case DIRT:
        fill(180, 150, 80);
        break;
      case ROCK:
        stroke(250);
        fill(100);
        break;
    }

    rect(this.pos.x, this.pos.y, gridSize, gridSize);
    pop();
  }

  empty() {
    return this.s === GAS;
  }
}
