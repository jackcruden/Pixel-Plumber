class Player {
  constructor(x, y) {
    this.pos = createVector(x, y);
    this.vel = createVector(0, 0);
    this.dir = createVector(0, 0);
    this.maxSpeed = 1.4;
    this.distance = 25;
    this.mult = 0.4;
  }

  move(mvmt) {
    if (mvmt) {
      this.vel.add(mvmt);
    }

    this.vel.mult(0.94);
    this.pos.add(this.vel);
  }

  collision(p) {
    for (let i = 0; i < p.length; i++) {
      for (let j = 0; j < p[0].length; j++) {
        let actualDist = dist(i * gridSize + gridSize / 2, j * gridSize + gridSize / 2, this.pos.x, this.pos.y);
        if (p[i][j].s === SOLID) {
          if (actualDist < this.distance) {
            let rebound = createVector(this.pos.x - i * gridSize, this.pos.y - j * gridSize);
            rebound.normalize();
            rebound.mult(1);
            this.vel.normalize();
            this.vel.add(rebound);
          }
        }
      }
    }
  }

  draw() {
    this.move();

    if (this.vel.mag() > this.maxSpeed) {
      this.vel.normalize();
      this.vel.mult(this.maxSpeed);
    }

    let newDir = createVector(mouseX - this.pos.x, mouseY - this.pos.y);
    newDir.normalize();
    newDir.mult(0.1);
    this.dir.add(newDir);
    this.dir.normalize();

    if (this.pos.x < 0) this.pos.x = width;
    if (this.pos.x > width) this.pos.x = 0;
    if (this.pos.y < 0) this.pos.y = height;
    if (this.pos.y > height) this.pos.y = 0;

    push();
    noStroke();
    fill(0);

    push();
    translate(this.pos.x, this.pos.y);
    rotate(this.dir.heading());
    rect(-gridSize, -gridSize, gridSize * 2, gridSize * 2);
    rect(gridSize, -gridSize / 4, gridSize / 2, gridSize / 2);
    pop();

    pop();
  }
}
