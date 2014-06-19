/*
// Some of the level generation code has been taken from
// a tutorial by Michael Cook from Tutsplus.com: http://goo.gl/0iDN1n
*/

/*
// The complex code I wrote based on the flood fill algorithm was supposed
// to check that the randomly generated levels were possible. However,
// there was a very strange bug that meant that after a few levels it would
// no longer accept any levels as possible. I've spent days and nights trying
// to make that work. I ended up settling for a boring solution; the area
// immediately surrounding the pipes is cleared of all solid pixels (dirt, rock).
// Unfortunately, this technically means that impossible levels could be
// presented to the user. However, I sat for 10 minutes and watched it generate
// ~500 levels they were all possible to complete. So, it should be all good!
*/

class Level {
  Pixel[][] ps; // Pixels array
  Pipe[] pipes; // Pipes array

  // Dimensions
  int dimX;
  int dimY;

  // For level generation
  float dirtChance = 0.44;
  int dirtBirthLimit = 4;
  int dirtDeathLimit = 3;
  int dirtNumberOfSteps = 10;
  
  float rockChance = 0.4;
  int rockBirthLimit = 4;
  int rockDeathLimit = 4;
  int rockNumberOfSteps = 10;
  
  float acidChance = 0.35;
  int acidBirthLimit = 4;
  int acidDeathLimit = 4;
  int acidNumberOfSteps = 6;
  
  // To keep track of health decreases
  // (when water touches acid)
  boolean healthDecrease = false;

  Level(int x, int y) {
    this.dimX = x;
    this.dimY = y;
        
    // Generate pipes at random x
    pipes = generatePipes();
    
    // Create dirt level
    Pixel[][] pD = generate();
    pD = populate(pD, solid, dirt, dirtChance);
    for (int i = 0; i < dirtNumberOfSteps; i++) {
      pD = generationStep(pD, solid, dirt, dirtBirthLimit, dirtDeathLimit);
    }
    
    // Create acid level
    Pixel[][] pA = generate();
    pA = populate(pA, liquid, acid, acidChance);
    for (int i = 0; i < acidNumberOfSteps; i++) {
      pA = generationStep(pA, liquid, acid, acidBirthLimit, acidDeathLimit);
    }
    
    // Combine dirt and acid
    ps = combine(pD, pA);
    
    println("combined dirt and acid");
    
    // Create rock level (make sure it's possible)
    Pixel[][] pR = generate();
    //do {
      // Generate rock
      println("generating rock");
      pR = populate(pR, solid, rock, rockChance);
      for (int i = 0; i < rockNumberOfSteps; i++) {
        println("iterating rock " + i);
        pR = generationStep(pR, solid, rock, rockBirthLimit, rockDeathLimit);
      }
      
      // Run flood fill algorithm
      //println("checking if possible");
      //println(isPossible(pR));
      
      // If it's not possible then change the seed and try again
//      long r = millis();
//      println("randomising seed " + r);
//      randomSeed(r);
    //} while (!isPossible); // Perform check
    
    // Overlay rock onto dirt
    ps = combine(ps, pR);
    
    // Clear area immediately around pipes
    for (int i = 0; i < dimX; i++) {
      for (int j = 0; j < dimY; j++) {
        // Add dirt first to keep water contained at source pipe
        if ((ps[i][j].s == gas || ps[i][j].e == acid) && dist(i, j, pipes[0].x+2, pipes[0].y) < 10) {
          ps[i][j].s = solid;
          ps[i][j].e = dirt;
        }
        if (ps[i][j].s == solid && dist(i, j, pipes[0].x+2, pipes[0].y) < 6) {
          ps[i][j].s = gas;
          ps[i][j].e = air;
        }
        if (ps[i][j].s == solid && dist(i, j, pipes[1].x+2, pipes[1].y) < 6) {
          ps[i][j].s = gas;
          ps[i][j].e = air;
        }
      }
    }

    // Generate the rock border
    ps = generateBorder(ps);
  }

  // Generate the empty level
  Pixel[][] generate() {
    Pixel[][] p = new Pixel[dimX][dimY];

    // Loop through each pixel and give it a state and element
    for (int i = 0; i < dimX; i++) {
      for (int j = 0; j < dimY; j++) {
        p[i][j] = new Pixel(new PVector(i * size, j * size), gas, air);
      }
    }

    return p;
  }

  // Populate the level
  Pixel[][] populate(Pixel[][] p, int state, int element, float chance) {
    // Loop through each pixel and give it a state and element
    for (int i = 0; i < dimX; i++) {
      for (int j = 0; j < dimY; j++) {
        // Default
        int s = gas;
        int e = air;

        // Chance of dirt
        if (random(1) < chance) {
          s = state;
          e = element;
        }

        p[i][j] = new Pixel(new PVector(i * size, j * size), s, e);
      }
    }

    return p;
  }

  Pixel[][] generationStep(Pixel[][] p, int state, int element, int birthLimit, int deathLimit) {
    Pixel[][] psNew = generate();
    // Loop over each row and column of the map
    for (int x=0; x<p.length; x++) {
      for (int y=0; y<p[0].length; y++) {
        int nbs = countAliveNeighbours(p, x, y);
        // The new value is based on our simulation rules
        // First, if a cell is alive but has too few neighbours, kill it.
        if (p[x][y].s == state && p[x][y].e == element) {
          if (nbs < deathLimit) {
            psNew[x][y].s = gas;
            psNew[x][y].e = air;
          } else {
            psNew[x][y].s = state;
            psNew[x][y].e = element;
          }
        } // Otherwise, if the cell is dead now, check if it has the right number of neighbours to be 'born'
        else {
          if (nbs > birthLimit) {
            psNew[x][y].s = state;
            psNew[x][y].e = element;
          } else {
            psNew[x][y].s = gas;
            psNew[x][y].e = air;
          }
        }
      }
    }
    
    return psNew;
  }

  // Returns the number of cells in a ring around (x,y) that are alive.
  int countAliveNeighbours(Pixel[][] p, int x, int y) {
    int count = 0;
    for (int i=-1; i<2; i++) {
      for (int j=-1; j<2; j++) {
        int neighbour_x = x+i;
        int neighbour_y = y+j;
        //If we're looking at the middle point
        if (i == 0 && j == 0) {
          //Do nothing, we don't want to add ourselves in!
        }
        //In case the index we're looking at it off the edge of the map
        else if (neighbour_x < 0 || neighbour_y < 0 || neighbour_x >= p.length || neighbour_y >= p[0].length) {
          count = count + 1;
        }
        //Otherwise, a normal check of the neighbour
        else if (!p[neighbour_x][neighbour_y].empty()) {
          count = count + 1;
        }
      }
    }

    return count;
  }
  
  // Combine two arrays ignoring gas/air
  // p1 is placed firt, then all non gas p2 pixels are placed on top
  Pixel[][] combine(Pixel[][] p1, Pixel[][] p2) {
    for (int x = 0; x < dimX; x++) {
      for (int y = 0; y < dimY; y++) {
        if (p2[x][y].s != gas && p2[x][y].e != air) {
          p1[x][y].s = p2[x][y].s;
          p1[x][y].e = p2[x][y].e;
        }
      }
    }
    
    return p1;
  }

  // Generate the rock border
  Pixel[][] generateBorder(Pixel[][] p) {
    // Generate rock border
    for (int x = 0; x < dimX; x++) {
      for (int y = 0; y < dimY; y++) {
        if (x == 0 || x == dimX-1 || y == 0 || y == dimY-1) {
          p[x][y].s = solid;
          p[x][y].e = rock;
        }
      }
    }

    return p;
  }
  
  // Generate pipes
  Pipe[] generatePipes() {
    Pipe[] p = new Pipe[2];
    p[0] = new Pipe((int)random(2, dimX - 8), 2, false, true);
    p[1] = new Pipe((int)random(2, dimX - 8), dimY-4, true, false);
    
    return p;
  }
  
  // Determine if the level is possible to complete
//  int[][] d = new int[dimX][dimY]; // Data array: 0 = air, 1 = rock, 2 = checked
//  boolean isPossible = false; // Possibility variable
//  boolean isPossible(Pixel[][] p) {
//    // This method uses the flood fill algorithm
//    // http://en.wikipedia.org/wiki/Flood_fill
//
//    // Create the data array
//    for (int i = 0; i < dimX; i++) {
//      for (int j = 0; j < dimY; j++) {
//        if (p[i][j].e == rock) {
//          d[i][j] = 1;
//        } else {
//          d[i][j] = 0;
//        }
//      }
//    }
//    
//    // Call the recursive method starting at pipe[0]
//    checkPixel(((pipes[0].x+3)/size), (pipes[0].y+2)/size);
//    
//    return isPossible;
//  }
  
  // This method gets called recursively, initially by isPossible()
//  void checkPixel(int x, int y) {
//    // Data array: 0 = air, 1 = rock, 2 = checked
//
//    // Check pixel to see if it is at end pipe, if so, return true
//    for (int p = 0; p < 4; p++) {
//      if (x == pipes[1].x + p && y == (pipes[1].y)) {
//        isPossible = true;
//        return;
//      }
//    }
//    
//    // If pixel already checked (2), return
//    if (d[x][y] == 2) return;
//    
//    // If rock (1), return
//    if (d[x][y] == 1) return;
//    
//    // Change gas (0) to checked (2)
//    if (d[x][y] == 0) d[x][y] = 2;
//    
//    // Print current array
////    if (random(100) > 99.9) {
////      println("DATA ARRAY:");
////      for (int i = 0; i < dimX; i++) {
////        for (int j = 0; j < dimY; j++) {
////          print(d[i][j]);
////        }
////        println("");
////      }
////    }
//    
//    // Check neighbours
//    if (y-1 > 0) checkPixel(x, y-1); // North
//    if (x+1 < dimX) checkPixel(x+1, y); // East
//    if (y+1 < dimY) checkPixel(x, y+1); // South
//    if (x-1 > 0) checkPixel(x-1, y); // West
//  }

  // Checks if a bullet has hit anything solid, destroys it
  // This method is likely overly complex
  ArrayList<Shot> shotCollision(ArrayList<Shot> shots, PVector pos) {
    for (int i = 0; i < dimX; i++) {
      for (int j = 0; j < dimY; j++) {
        for (Shot s : shots) {
          if (s.active && ps[i][j].s == solid && dist(i*size+size/2, j*size+size/2, s.pos.x, s.pos.y) < 15) {
            s.active = false;
            // Clear dirt, or place dirt
            for (int k = 0; k < dimX; k++) {
              for (int l = 0; l < dimY; l++) {
                if (dist(k*size, l*size, s.pos.x, s.pos.y) < 30) {
                  if (ps[k][l].e == dirt && s.type == LEFT) {
                    ps[k][l].s = gas;
                    ps[k][l].e = air;
                  } else if (ps[k][l].e != rock && s.type == RIGHT && dist(pos.x, pos.y, s.pos.x, s.pos.y) > 50) {
                    ps[k][l].s = solid;
                    ps[k][l].e = dirt;
                  }
                }
              }
            }
          }
        }
      }
    }
    
    return shots;
  }

  void update() {
    // Create temp pixels
    Pixel[][] psTemp = generate();
    psTemp = populate(psTemp, gas, air, 1);
    for (int i = 0; i < dimX; i++) {
      for (int j = 0; j < dimY; j++) {
        psTemp[i][j] = new Pixel(new PVector(i*size, j*size), ps[i][j].s, ps[i][j].e);
      }
    }

    // Movement
    for (int i = 0; i < dimX; i++) {
      for (int j = 0; j < dimY; j++) {
        // Get current state
        int thisState = ps[i][j].s;
        int thisElement = ps[i][j].e;
        
        // Check if Pixel is a Pipe Pixel, if so turn all
        // near pixels to water/air for top/bottom pipes
        if (i == pipes[0].x && j == pipes[0].y+1) {
          // Source pipe
          if (frameCount % 5 == 0) {
            for (int k = 2; k < 4; k++) {
              psTemp[i+k][j+1].s = liquid;
              psTemp[i+k][j+1].e = water;
            }
          }
        } else if (i == pipes[1].x-1 && j == pipes[1].y-1) {
          // Intake pipe
          for (int k = 2; k < 6; k++) {
            if (psTemp[i+k][j+1].e == water) pipes[1].active = true;
            psTemp[i+k][j].s = gas;
            psTemp[i+k][j].e = air;
            psTemp[i+k][j-1].s = gas;
            psTemp[i+k][j-1].e = air;
          }
        }

        // If liquid
        if (thisState == liquid) {
          // Gravity
          if (j+1 < dimY && ps[i][j+1].empty()) {
            psTemp[i][j+1].s = thisState;
            psTemp[i][j+1].e = thisElement;
            psTemp[i][j].s = gas;
            psTemp[i][j].e = air;
          }

          // Check if acid touching water
          if (thisElement == acid) {
            if (j+1 < dimY && ps[i][j+1].e == water) {
              psTemp[i][j].s = gas;
              psTemp[i][j].e = air;
              psTemp[i][j+1].s = gas;
              psTemp[i][j+1].e = air;
              ps[i][j].s = gas;
              ps[i][j].e = air;
              ps[i][j+1].s = gas;
              ps[i][j+1].e = air;
              healthDecrease = true;
            } else if (j-1 < dimY && ps[i][j-1].e == water) {
              psTemp[i][j].s = gas;
              psTemp[i][j].e = air;
              psTemp[i][j-1].s = gas;
              psTemp[i][j-1].e = air;
              ps[i][j].s = gas;
              ps[i][j].e = air;
              ps[i][j-1].s = gas;
              ps[i][j-1].e = air;
              healthDecrease = true;
            } else if (i-1 > 0 && ps[i-1][j].e == water) {
              psTemp[i][j].s = gas;
              psTemp[i][j].e = air;
              psTemp[i-1][j].s = gas;
              psTemp[i-1][j].e = air;
              ps[i][j].s = gas;
              ps[i][j].e = air;
              ps[i-1][j].s = gas;
              ps[i-1][j].e = air;
              healthDecrease = true;
            } else if (i+1 > 0 && ps[i+1][j].e == water) {
              psTemp[i][j].s = gas;
              psTemp[i][j].e = air;
              psTemp[i+1][j].s = gas;
              psTemp[i+1][j].e = air;
              ps[i][j].s = gas;
              ps[i][j].e = air;
              ps[i+1][j].s = gas;
              ps[i+1][j].e = air;
              healthDecrease = true;
            }
          }

          // Lateral movement
          if (thisState == liquid) {
            if (i-2 >= 0 && i+2 < dimX && j+1 < dimY && j-1 >= 0 && !ps[i][j+1].empty()) {
              if (random(0, 1) > 0.5) {
                // Left movement
                if (ps[i-1][j].empty() && ps[i-1][j-1].s != liquid && psTemp[i-1][j].s != liquid) {
                  psTemp[i-1][j].s = thisState;
                  psTemp[i-1][j].e = thisElement;
                  psTemp[i][j].s = gas;
                  psTemp[i][j].e = air;
                }
              } else {
                // Right movement
                if (ps[i+1][j].empty() && ps[i+1][j-1].s != liquid && psTemp[i+1][j].s != liquid) {
                  psTemp[i+1][j].s = thisState;
                  psTemp[i+1][j].e = thisElement;
                  psTemp[i][j].s = gas;
                  psTemp[i][j].e = air;
                }
              }
            }
          }
        }
      }
    }

    // Restore array
    ps = psTemp;
  }

  void draw() {
    // Counts (for debug)
    int airCount = 0;
    int waterCount = 0;
    int acidCount = 0;
    int dirtCount = 0;
    int rockCount = 0;

    // Loop pixels
    for (int i = 0; i < dimX; i++) {
      for (int j = 0; j < dimY; j++) {
        if (ps[i][j].e == air) airCount++;
        if (ps[i][j].e == water) waterCount++;
        if (ps[i][j].e == acid) acidCount++;
        if (ps[i][j].e == dirt) dirtCount++;
        if (ps[i][j].e == rock) rockCount++;
        
        // Draw pixels
        ps[i][j].draw();
      }
    }
    
    // Print debug info
//    text(airCount + " air", 8, 40);
//    text(waterCount + " water", 8, 60);
//    text(acidCount + " acid", 8, 80);
//    text(dirtCount + " dirt", 8, 100);
//    text(rockCount + " rock", 8, 120);
  }
  
  // Draw pipes
  void drawPipes() {
    pipes[0].draw();
    pipes[1].draw();
  }
  
  
}

