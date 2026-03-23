class Game {
  constructor() {
    // Game states
    this.TITLE = 0;
    this.PLAY = 1;
    this.DEATH = 2;
    this.CONTROLS = 3;

    this.level = null;
    this.player = null;
    this.shots = [];
    this.controlsImage = null;

    // Key states
    this.keyup = false;
    this.keyright = false;
    this.keyleft = false;
    this.keydown = false;

    // Game manager variables
    this.levelNum = 0;
    this.health = 15;
    this.highscore = 0;

    // Title fade
    this.fade = 0;

    this.state = this.TITLE;
    this.constructorTitle();
  }

  constructorTitle(dead) {
    this.state = this.TITLE;
    if (dead) this.state = this.DEATH;

    this.fade = 0;

    if (this.state !== this.DEATH) {
      this.level = new Level(numX, numY);
    }
  }

  constructorPlay() {
    if (this.state !== this.PLAY) {
      this.health = 15;
      if (this.levelNum > this.highscore) this.highscore = this.levelNum;
      this.levelNum = 0;
      this.state = this.PLAY;
    }

    this.levelNum++;

    this.level = new Level(numX, numY - 4);
    console.log('level created');

    this.player = new Player((this.level.pipes[0].x + 3) * gridSize, gridSize * 3);
    console.log('player created');
    this.player.vel.add(createVector(0, gridSize / 2.5));
    this.shots = [];
  }

  constructorControls() {
    this.state = this.CONTROLS;
    this.controlsImage = loadImage('Pixel_Plumber/data/controls.png');
  }

  draw() {
    if (this.state === this.TITLE || this.state === this.DEATH) {
      this.level.update();
      this.level.draw();
      this.level.drawPipes();
      if (this.state === this.DEATH) {
        this.drawPanel();
      }

      if (this.fade < 255) this.fade += 2;

      noStroke();
      fill(0, this.fade * 0.6);
      rect(0, 0, width, height);

      // Title
      fill(255, this.fade);
      rect(70, 50, 900, 140);
      textSize(100);
      fill(100, this.fade);
      text('Pixel Plumber', 100, 150);

      // Show score from death
      if (this.state === this.DEATH) {
        fill(255, this.fade);
        rect(250, 230, 550, 40);
        textSize(30);
        fill(230, 80, 80, this.fade);
        text('You died. Your score was ' + this.levelNum + '.', 260, 258);
      }

      // New highscore?
      if (this.levelNum > this.highscore) {
        fill(255, this.fade);
        rect(250, 270, 550, 30);
        textSize(30);
        fill(80, 230, 80, this.fade);
        text("That's a new highscore!", 310, 290);
      }

      // High score
      fill(255, this.fade);
      rect(430, 350, 200, 100);
      textSize(30);
      fill(100, this.fade);
      text('Highscore', 440, 380);
      textSize(60);
      if (this.levelNum > this.highscore) {
        text(this.levelNum, 440, 435);
      } else {
        text(this.highscore, 440, 435);
      }

      // Play button
      fill(255, this.fade);
      if (this.mouseOver(280, 550, 200, 100)) {
        fill(200, this.fade);
      }
      rect(280, 550, 200, 100);
      textSize(30);
      fill(100, this.fade);
      text('Play', 335, 608);

      // Controls button
      fill(255, this.fade);
      if (this.mouseOver(580, 550, 200, 100)) {
        fill(200, this.fade);
      }
      rect(580, 550, 200, 100);
      textSize(30);
      fill(100, this.fade);
      text('Controls', 597, 608);
    } else if (this.state === this.PLAY) {
      // Player movement
      if (keyIsPressed) {
        if (this.keyup) game.player.move(createVector(0, -1));
        if (this.keyleft) game.player.move(createVector(-1, 0));
        if (this.keydown) game.player.move(createVector(0, 1));
        if (this.keyright) game.player.move(createVector(1, 0));
      }

      // Run player collision
      for (let c = 0; c < 10; c++) {
        this.player.collision(this.level.ps);
      }

      this.level.update();

      this.level.draw();
      this.player.draw();

      // Draw shots
      this.shots = this.level.shotCollision(this.shots, this.player.pos);
      for (let shot of this.shots) {
        if (shot.active) {
          shot.draw();
        }
      }

      // Draw pipes over player
      this.level.drawPipes();

      this.drawPanel();

      // Check if player at end pipe and end pipe active
      if (
        dist(this.player.pos.x, this.player.pos.y, (this.level.pipes[1].x + 2) * gridSize, this.level.pipes[1].y * gridSize) < 30 &&
        this.level.pipes[1].active
      ) {
        this.constructorPlay();
        console.log('new level');
        return;
      }

      // Check if health is depleted
      if (this.health < 0) {
        this.constructorTitle(true);
      }

      // Check if water has touched acid this tick
      if (this.level.healthDecrease) {
        this.health -= 0.04;
        this.level.healthDecrease = false;
      }
    } else if (this.state === this.CONTROLS) {
      if (this.controlsImage) {
        image(this.controlsImage, 0, 0);
      }
    }
  }

  drawPanel() {
    push();

    stroke(255);
    strokeWeight(0.001);

    for (let x = 0; x < numX; x++) {
      for (let y = numY - 5; y < numY; y++) {
        fill(100);
        rect(x * gridSize, y * gridSize, gridSize, gridSize);
      }
    }

    let posY = gridSize * numY - 60;

    fill(255);

    // Title
    textSize(70);
    text('Pixel Plumber', gridSize * 0.5, posY + gridSize * 3);

    // Levels
    textSize(20);
    text('Level', gridSize * 43, posY + gridSize);
    textSize(40);
    text(this.levelNum, gridSize * 43, posY + gridSize * 3);

    // Health
    textSize(20);
    text('Water health', gridSize * 50, posY + gridSize);
    rect(gridSize * 50, posY + gridSize + gridSize / 3, gridSize * 16, gridSize * 1.7);
    fill(255 - 16 * this.health, 16 * this.health, 0);
    rect(gridSize * 50 + gridSize / 2, posY + gridSize * 1.8, gridSize * this.health, gridSize * 0.8);

    pop();
  }

  mousePressed(button) {
    if (this.state === this.TITLE || this.state === this.DEATH) {
      if (this.mouseOver(280, 550, 200, 100)) {
        this.constructorPlay();
      } else if (this.mouseOver(580, 550, 200, 100)) {
        this.constructorControls();
      }
    } else if (this.state === this.PLAY) {
      this.shots.push(new Shot(button));
    } else if (this.state === this.CONTROLS) {
      this.constructorTitle();
    }
  }

  keyPressed(k) {
    if (k === 'w') this.keyup = true;
    if (k === 's') this.keydown = true;
    if (k === 'a') this.keyleft = true;
    if (k === 'd') this.keyright = true;
    if (k === 'm') this.health--;
  }

  keyReleased(k) {
    if (k === 'w') this.keyup = false;
    if (k === 's') this.keydown = false;
    if (k === 'a') this.keyleft = false;
    if (k === 'd') this.keyright = false;
  }

  mouseOver(x, y, w, h) {
    return (x < mouseX && mouseX < x + w && y < mouseY && mouseY < y + h);
  }
}
