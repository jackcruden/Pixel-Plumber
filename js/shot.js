class Shot {
  constructor(type) {
    this.pos = createVector(game.player.pos.x, game.player.pos.y);
    this.vel = createVector(mouseX - game.player.pos.x, mouseY - game.player.pos.y);
    this.type = type;
    this.active = true;

    this.vel.normalize();
    this.vel.mult(10);

    this.pos.add(this.vel);
  }

  draw() {
    push();

    this.pos.add(this.vel);

    fill(0);
    noStroke();

    push();
    translate(this.pos.x, this.pos.y);
    rotate(this.vel.heading());
    rect(-gridSize / 4, -gridSize / 4, gridSize / 2, gridSize / 2);
    pop();

    pop();
  }
}
