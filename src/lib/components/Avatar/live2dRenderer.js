import { Application } from 'pixi.js';
import { Live2DSprite, Config } from 'easy-live2d';

Config.CubismLoggingLevel = 2; // LogLevel_Warning
Config.MouseFollow = false; // we drive parameters manually from avatar_input

export class Live2DRenderer {
  constructor() {
    this._app = null;
    this._sprite = null;
    this._canvas = null;
    this._destroyed = false;
  }

  async init(canvas) {
    if (this._destroyed) return;
    this._canvas = canvas;

    this._app = new Application();
    await this._app.init({
      canvas,
      backgroundAlpha: 0,
      autoDensity: true,
      resolution: Math.max(window.devicePixelRatio || 1, 1),
      antialias: true,
    });
  }

  async loadModel(modelPath) {
    if (this._destroyed) return;
    this.destroyModel();

    this._sprite = new Live2DSprite({
      modelPath,
      draggable: false,
    });

    this._app.stage.addChild(this._sprite);
    this.fitSprite();

    await this._sprite.ready;
  }

  fitSprite() {
    if (!this._sprite || !this._app) return;

    const canvasW = this._canvas?.clientWidth || 612;
    const canvasH = this._canvas?.clientHeight || 354;

    this._sprite.width = canvasW;
    this._sprite.height = canvasH;
  }

  resize(width, height) {
    if (!this._app) return;

    this._app.renderer.resize(width, height);
    this.fitSprite();
  }

  setParameterValue(id, value) {
    if (!this._sprite) return;
    this._sprite.setParameterValueById(id, value);
  }

  async startMotion(group, no, priority = 2) {
    if (!this._sprite) return;
    await this._sprite.startMotion({ group, no, priority });
  }

  setExpression(expressionId) {
    if (!this._sprite) return;
    this._sprite.setExpression({ expressionId });
  }

  setExpressionByIndex(index) {
    if (!this._sprite) return;
    this._sprite.setExpression({ index });
  }

  getExpressions() {
    if (!this._sprite) return [];
    return this._sprite.getExpressions();
  }

  destroyModel() {
    if (this._sprite) {
      this._sprite.destroy();
      this._sprite = null;
    }
    if (this._app?.stage) {
      this._app.stage.removeChildren();
    }
  }

  destroy() {
    this._destroyed = true;
    this.destroyModel();
    if (this._app) {
      this._app.destroy(true);
      this._app = null;
    }
    this._canvas = null;
  }
}
