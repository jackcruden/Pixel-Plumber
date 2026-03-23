class Level {
  constructor(dimX, dimY) {
    this.dimX = dimX;
    this.dimY = dimY;

    // Level generation parameters
    this.dirtChance = 0.44;
    this.dirtBirthLimit = 4;
    this.dirtDeathLimit = 3;
    this.dirtNumberOfSteps = 10;

    this.rockChance = 0.4;
    this.rockBirthLimit = 4;
    this.rockDeathLimit = 4;
    this.rockNumberOfSteps = 10;

    this.acidChance = 0.35;
    this.acidBirthLimit = 4;
    this.acidDeathLimit = 4;
    this.acidNumberOfSteps = 6;

    this.healthDecrease = false;

    // Generate pipes
    this.pipes = this.generatePipes();

    // Create dirt level
    let pD = this.generate();
    pD = this.populate(pD, SOLID, DIRT, this.dirtChance);
    for (let i = 0; i < this.dirtNumberOfSteps; i++) {
      pD = this.generationStep(pD, SOLID, DIRT, this.dirtBirthLimit, this.dirtDeathLimit);
    }

    // Create acid level
    let pA = this.generate();
    pA = this.populate(pA, LIQUID, ACID, this.acidChance);
    for (let i = 0; i < this.acidNumberOfSteps; i++) {
      pA = this.generationStep(pA, LIQUID, ACID, this.acidBirthLimit, this.acidDeathLimit);
    }

    // Combine dirt and acid
    this.ps = this.combine(pD, pA);

    // Create rock level
    let pR = this.generate();
    pR = this.populate(pR, SOLID, ROCK, this.rockChance);
    for (let i = 0; i < this.rockNumberOfSteps; i++) {
      pR = this.generationStep(pR, SOLID, ROCK, this.rockBirthLimit, this.rockDeathLimit);
    }

    // Overlay rock onto dirt
    this.ps = this.combine(this.ps, pR);

    // Clear area immediately around pipes
    for (let i = 0; i < this.dimX; i++) {
      for (let j = 0; j < this.dimY; j++) {
        if ((this.ps[i][j].s === GAS || this.ps[i][j].e === ACID) && dist(i, j, this.pipes[0].x + 2, this.pipes[0].y) < 10) {
          this.ps[i][j].s = SOLID;
          this.ps[i][j].e = DIRT;
        }
        if (this.ps[i][j].s === SOLID && dist(i, j, this.pipes[0].x + 2, this.pipes[0].y) < 6) {
          this.ps[i][j].s = GAS;
          this.ps[i][j].e = AIR;
        }
        if (this.ps[i][j].s === SOLID && dist(i, j, this.pipes[1].x + 2, this.pipes[1].y) < 6) {
          this.ps[i][j].s = GAS;
          this.ps[i][j].e = AIR;
        }
      }
    }

    // Generate the rock border
    this.ps = this.generateBorder(this.ps);
  }

  generate() {
    let p = [];
    for (let i = 0; i < this.dimX; i++) {
      p[i] = [];
      for (let j = 0; j < this.dimY; j++) {
        p[i][j] = new Pixel(createVector(i * gridSize, j * gridSize), GAS, AIR);
      }
    }
    return p;
  }

  populate(p, state, element, chance) {
    for (let i = 0; i < this.dimX; i++) {
      for (let j = 0; j < this.dimY; j++) {
        let s = GAS;
        let e = AIR;

        if (random(1) < chance) {
          s = state;
          e = element;
        }

        p[i][j] = new Pixel(createVector(i * gridSize, j * gridSize), s, e);
      }
    }
    return p;
  }

  generationStep(p, state, element, birthLimit, deathLimit) {
    let psNew = this.generate();
    for (let x = 0; x < p.length; x++) {
      for (let y = 0; y < p[0].length; y++) {
        let nbs = this.countAliveNeighbours(p, x, y);
        if (p[x][y].s === state && p[x][y].e === element) {
          if (nbs < deathLimit) {
            psNew[x][y].s = GAS;
            psNew[x][y].e = AIR;
          } else {
            psNew[x][y].s = state;
            psNew[x][y].e = element;
          }
        } else {
          if (nbs > birthLimit) {
            psNew[x][y].s = state;
            psNew[x][y].e = element;
          } else {
            psNew[x][y].s = GAS;
            psNew[x][y].e = AIR;
          }
        }
      }
    }
    return psNew;
  }

  countAliveNeighbours(p, x, y) {
    let count = 0;
    for (let i = -1; i < 2; i++) {
      for (let j = -1; j < 2; j++) {
        let nx = x + i;
        let ny = y + j;
        if (i === 0 && j === 0) {
          // skip self
        } else if (nx < 0 || ny < 0 || nx >= p.length || ny >= p[0].length) {
          count++;
        } else if (!p[nx][ny].empty()) {
          count++;
        }
      }
    }
    return count;
  }

  combine(p1, p2) {
    for (let x = 0; x < this.dimX; x++) {
      for (let y = 0; y < this.dimY; y++) {
        if (p2[x][y].s !== GAS && p2[x][y].e !== AIR) {
          p1[x][y].s = p2[x][y].s;
          p1[x][y].e = p2[x][y].e;
        }
      }
    }
    return p1;
  }

  generateBorder(p) {
    for (let x = 0; x < this.dimX; x++) {
      for (let y = 0; y < this.dimY; y++) {
        if (x === 0 || x === this.dimX - 1 || y === 0 || y === this.dimY - 1) {
          p[x][y].s = SOLID;
          p[x][y].e = ROCK;
        }
      }
    }
    return p;
  }

  generatePipes() {
    let p = [];
    p[0] = new Pipe(floor(random(2, this.dimX - 8)), 2, false, true);
    p[1] = new Pipe(floor(random(2, this.dimX - 8)), this.dimY - 4, true, false);
    return p;
  }

  shotCollision(shots, pos) {
    for (let i = 0; i < this.dimX; i++) {
      for (let j = 0; j < this.dimY; j++) {
        for (let s of shots) {
          if (s.active && this.ps[i][j].s === SOLID && dist(i * gridSize + gridSize / 2, j * gridSize + gridSize / 2, s.pos.x, s.pos.y) < 15) {
            s.active = false;
            for (let k = 0; k < this.dimX; k++) {
              for (let l = 0; l < this.dimY; l++) {
                if (dist(k * gridSize, l * gridSize, s.pos.x, s.pos.y) < 30) {
                  if (this.ps[k][l].e === DIRT && s.type === LEFT) {
                    this.ps[k][l].s = GAS;
                    this.ps[k][l].e = AIR;
                  } else if (this.ps[k][l].e !== ROCK && s.type === RIGHT && dist(pos.x, pos.y, s.pos.x, s.pos.y) > 50) {
                    this.ps[k][l].s = SOLID;
                    this.ps[k][l].e = DIRT;
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

  update() {
    let psTemp = this.generate();
    for (let i = 0; i < this.dimX; i++) {
      for (let j = 0; j < this.dimY; j++) {
        psTemp[i][j] = new Pixel(createVector(i * gridSize, j * gridSize), this.ps[i][j].s, this.ps[i][j].e);
      }
    }

    for (let i = 0; i < this.dimX; i++) {
      for (let j = 0; j < this.dimY; j++) {
        let thisState = this.ps[i][j].s;
        let thisElement = this.ps[i][j].e;

        // Pipe logic
        if (i === this.pipes[0].x && j === this.pipes[0].y + 1) {
          if (frameCount % 5 === 0) {
            for (let k = 2; k < 4; k++) {
              psTemp[i + k][j + 1].s = LIQUID;
              psTemp[i + k][j + 1].e = WATER;
            }
          }
        } else if (i === this.pipes[1].x - 1 && j === this.pipes[1].y - 1) {
          for (let k = 2; k < 6; k++) {
            if (psTemp[i + k][j + 1].e === WATER) this.pipes[1].active = true;
            psTemp[i + k][j].s = GAS;
            psTemp[i + k][j].e = AIR;
            psTemp[i + k][j - 1].s = GAS;
            psTemp[i + k][j - 1].e = AIR;
          }
        }

        // Liquid physics
        if (thisState === LIQUID) {
          // Gravity
          if (j + 1 < this.dimY && this.ps[i][j + 1].empty()) {
            psTemp[i][j + 1].s = thisState;
            psTemp[i][j + 1].e = thisElement;
            psTemp[i][j].s = GAS;
            psTemp[i][j].e = AIR;
          }

          // Acid-water interaction
          if (thisElement === ACID) {
            if (j + 1 < this.dimY && this.ps[i][j + 1].e === WATER) {
              psTemp[i][j].s = GAS; psTemp[i][j].e = AIR;
              psTemp[i][j + 1].s = GAS; psTemp[i][j + 1].e = AIR;
              this.ps[i][j].s = GAS; this.ps[i][j].e = AIR;
              this.ps[i][j + 1].s = GAS; this.ps[i][j + 1].e = AIR;
              this.healthDecrease = true;
            } else if (j - 1 < this.dimY && this.ps[i][j - 1].e === WATER) {
              psTemp[i][j].s = GAS; psTemp[i][j].e = AIR;
              psTemp[i][j - 1].s = GAS; psTemp[i][j - 1].e = AIR;
              this.ps[i][j].s = GAS; this.ps[i][j].e = AIR;
              this.ps[i][j - 1].s = GAS; this.ps[i][j - 1].e = AIR;
              this.healthDecrease = true;
            } else if (i - 1 > 0 && this.ps[i - 1][j].e === WATER) {
              psTemp[i][j].s = GAS; psTemp[i][j].e = AIR;
              psTemp[i - 1][j].s = GAS; psTemp[i - 1][j].e = AIR;
              this.ps[i][j].s = GAS; this.ps[i][j].e = AIR;
              this.ps[i - 1][j].s = GAS; this.ps[i - 1][j].e = AIR;
              this.healthDecrease = true;
            } else if (i + 1 > 0 && this.ps[i + 1][j].e === WATER) {
              psTemp[i][j].s = GAS; psTemp[i][j].e = AIR;
              psTemp[i + 1][j].s = GAS; psTemp[i + 1][j].e = AIR;
              this.ps[i][j].s = GAS; this.ps[i][j].e = AIR;
              this.ps[i + 1][j].s = GAS; this.ps[i + 1][j].e = AIR;
              this.healthDecrease = true;
            }
          }

          // Lateral movement
          if (thisState === LIQUID) {
            if (i - 2 >= 0 && i + 2 < this.dimX && j + 1 < this.dimY && j - 1 >= 0 && !this.ps[i][j + 1].empty()) {
              if (random(0, 1) > 0.5) {
                if (this.ps[i - 1][j].empty() && this.ps[i - 1][j - 1].s !== LIQUID && psTemp[i - 1][j].s !== LIQUID) {
                  psTemp[i - 1][j].s = thisState;
                  psTemp[i - 1][j].e = thisElement;
                  psTemp[i][j].s = GAS;
                  psTemp[i][j].e = AIR;
                }
              } else {
                if (this.ps[i + 1][j].empty() && this.ps[i + 1][j - 1].s !== LIQUID && psTemp[i + 1][j].s !== LIQUID) {
                  psTemp[i + 1][j].s = thisState;
                  psTemp[i + 1][j].e = thisElement;
                  psTemp[i][j].s = GAS;
                  psTemp[i][j].e = AIR;
                }
              }
            }
          }
        }
      }
    }

    this.ps = psTemp;
  }

  draw() {
    for (let i = 0; i < this.dimX; i++) {
      for (let j = 0; j < this.dimY; j++) {
        this.ps[i][j].draw();
      }
    }
  }

  drawPipes() {
    this.pipes[0].draw();
    this.pipes[1].draw();
  }
}
