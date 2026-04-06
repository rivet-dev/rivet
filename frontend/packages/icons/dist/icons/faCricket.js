var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCricketBatBall.js
var require_faCricketBatBall = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCricketBatBall.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cricket-bat-ball";
    var width = 512;
    var height = 512;
    var aliases = [127951, "cricket"];
    var unicode = "f449";
    var svgPathData = "M502.6 9.4c12.5 12.5 12.5 32.8 0 45.3l-135 135L409 231c4.5 4.5 7 10.6 7 17s-2.5 12.5-7 17l-23 23L240 288c-8.8 0-16-7.2-16-16l0-145.9 23-23c9.4-9.4 24.6-9.4 33.9 0l41.4 41.4 135-135c12.5-12.5 32.8-12.5 45.3 0zM352 432a80 80 0 1 1 160 0 80 80 0 1 1 -160 0zM16.1 334L192 158.1 192 272c0 26.5 21.5 48 48 48l113.9 0L178 495.9c-10 10-24.4 16.7-40.4 15.6c-11.3-.8-28.6-2.8-46.4-8.2C73.9 498.2 54.1 489.1 39 473c-16.2-15-25.2-34.9-30.4-52.2C3.2 403 1.2 385.7 .5 374.4C-.6 358.4 6.1 344 16.1 334z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faCricketBatBall = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faCricket.js
var import_faCricketBatBall = __toESM(require_faCricketBatBall());
var export_faCricket = import_faCricketBatBall.definition;
export {
  export_faCricket as faCricket
};
