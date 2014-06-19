class Game {
  // Level object
  Level level;
  
  // Game states
  int title = 0;
  int play = 1;
  int death = 2;
  int controls = 3;
  int state = title;
  PImage controlsImage;
  
  // Player
  Player player;
  ArrayList<Shot> shots;
  // To detect if key held down
  boolean keyup = false;
  boolean keyright = false;
  boolean keyleft = false;
  boolean keydown = false;
  
  // Game manager variables
  int levelNum = 0;
  float health = 15;
  int highscore = 0;
  
  // Title fade
  float fade = 0;
  
  Game() {
    constructorTitle();
  }
  
  void constructorTitle() {
    constructorTitle(false);
  }
  void constructorTitle(boolean dead) {
    state = title;
    if (dead) state = death;
    
    // Reset the fade (for cool effect)
    fade = 0;
    
    // Create the background level object
    if (state != death) {
      this.level = new Level(numX, numY);
    }
  }
    
  void constructorPlay() {
    if (state != play) {
      health = 15;
      if (levelNum > highscore) highscore = levelNum;
      levelNum = 0;
      state = play;
    }
    
    // Increment the level counter
    levelNum++;
    
    // Create the level object
    this.level = new Level(numX, numY-4);
    println("level created");
    
    // Create the player
    this.player = new Player((level.pipes[0].x+3)*size, size*3);
    println("player created");
    this.player.vel.add(new PVector(0, size/2.5));
    this.shots = new ArrayList<Shot>();
  }
  
  void constructorControls() {
    state = controls;
    
    controlsImage = loadImage("controls.png");
  }
    
  void draw() {
    if (state == title || state == death) {
      // Draw level in background
      level.update();      
      level.draw();
      level.drawPipes();
      if (state == death) {
        drawPanel();
      }
      
      // Check title fade
      if (fade < 255) fade += 2;
      //fade = 255; // debug
      
      noStroke();
      fill(0, fade*0.6);
      rect(0, 0, width, height);
            
      // Title
      fill(255, fade); 
      rect(70, 50, 900, 140);
      textSize(100);
      fill(100, fade);
      text("Pixel Plumber", 100, 150);
      
      // Show score from death
      if (state == death) { // Change to death
        fill(255, fade); 
        rect(250, 230, 550, 40);
        textSize(30);
        fill(230, 80, 80, fade);
        text("You died. Your score was " + levelNum + ".", 260, 258);
      }
      
      // New highscore?
      if (levelNum > highscore) { // Change to >
        fill(255, fade); 
        rect(250, 270, 550, 30);
        textSize(30);
        fill(80, 230, 80, fade);
        text("That's a new highscore!", 310, 290);
      }
      
      // High score
      fill(255, fade); 
      rect(430, 350, 200, 100);
      textSize(30);
      fill(100, fade);
      text("Highscore", 440, 380);
      textSize(60);
      if (levelNum > highscore) {
        text(levelNum, 440, 435);
      } else {
        text(highscore, 440, 435);
      }
      
      // Play
      fill(255, fade);
      if (mouseOver(280, 550, 200, 100)) {
        fill(200, fade);
      }
      rect(280, 550, 200, 100);
      textSize(30);
      fill(100, fade);
      text("Play", 335, 608);
      
      // Controls
      fill(255, fade);
      if (mouseOver(580, 550, 200, 100)) {
        fill(200, fade);
      }
      rect(580, 550, 200, 100);
      textSize(30);
      fill(100, fade);
      text("Controls", 597, 608);
    } else if (state == play) {
      // Player movement
      if (keyPressed) {
        if (keyup) game.player.move(new PVector(0, -1)); // Up
        if (keyleft) game.player.move(new PVector(-1, 0)); // Left
        if (keydown) game.player.move(new PVector(0, 1)); // Down
        if (keyright) game.player.move(new PVector(1, 0)); // Right
      }
      
      // Run player collision a few times
      for (int c = 0; c < 10; c++) {
        player.collision(level.ps);
      }
      
      // Update level
      level.update();
      
      // Draw level and player
      level.draw();
      player.draw();
      
      // Draw shots
      shots = level.shotCollision(shots, player.pos);
      if (shots.size() > 0)  {
        for (Shot shot : shots) {
          if (shot.active) {
            shot.draw();
          }
        }
      }
      
      // Draw pipes (over top of player)
      level.drawPipes();
      
      // Draw gameplay stats
      drawPanel();
      
      // Check if player at end pipe and end pipe active
      if (dist(player.pos.x, player.pos.y, (level.pipes[1].x+2)*size, level.pipes[1].y*size) < 30 && level.pipes[1].active) {
        constructorPlay();
        println("new level");
        return;
      }
      
      // Check if helath is depleted, run death screen
      if (health < 0) {
        constructorTitle(true);
      }
      
      // Check if water has touched acid this tick
      if (level.healthDecrease) {
        health -= 0.04;
        level.healthDecrease = false;
      }
    } else if (state == controls) {
      image(controlsImage, 0, 0);
    }
  }
  
  void drawPanel() {
    pushStyle();
    
    // Style
    stroke(255);
    strokeWeight(0.001);
    
    for (int x = 0; x < numX; x++) {
      for (int y = numY-5; y < numY; y++) {
        fill(100);
        rect(x*size, y*size, size, size);
      }
    }
    
    // Position of panel
    float posY = size * numY-60;
    
    // Draw stats
    fill(255);
    
    // Title
    textSize(70);
    text("Pixel Plumber", size*0.5, posY+size*3);
    
    // Levels
    textSize(20);
    text("Level", size*43, posY+size);
    textSize(40);
    text(levelNum, size*43, posY+size*3);
    
    // Health
    textSize(20);
    text("Water health", size*50, posY+size);
    rect(size*50 + size*0, posY+size*1+size/3, size*16, size*1.7);
    fill(255-16*health, 16*health, 0);
    rect(size*50 + size/2, posY+size*1.8, size*health, size*0.8);
    
    popStyle();
  }
  
  void mousePressed(int mouseButton) {
    if (state == title || state == death) {
      if (mouseOver(280, 550, 200, 100)) {
        constructorPlay();
      } else if (mouseOver(580, 550, 200, 100)) {
        constructorControls();
      }
    } else if (state == play) {
      game.shots.add(new Shot(mouseButton));
    } else if (state == controls) {
      constructorTitle();
    }
  }
  
  void keyPressed(int key) {
    // To detect if key is being held down
    if (key == 'w') keyup = true; 
    if (key == 's') keydown = true; 
    if (key == 'a') keyleft = true; 
    if (key == 'd') keyright = true; 
    if (key == 'm') health--;
  }
  
  void keyReleased(int key) {
    if (key == 'w') keyup = false; 
    if (key == 's') keydown = false; 
    if (key == 'a') keyleft = false; 
    if (key == 'd') keyright = false; 
  }
  
  boolean mouseOver(int x, int y, int w, int h) {
    return (x < mouseX && mouseX < x+w && y < mouseY && mouseY < y+h);
  }
}
