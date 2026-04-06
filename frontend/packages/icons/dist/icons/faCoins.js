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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCoins.js
var require_faCoins = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCoins.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "coins";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f51e";
    var svgPathData = "M128 96l0-16c0-44.2 86-80 192-80S512 35.8 512 80l0 16c0 30.6-41.3 57.2-102 70.7-2.4-2.8-4.9-5.5-7.4-8-15.5-15.3-35.5-26.9-56.4-35.5-41.9-17.5-96.5-27.1-154.2-27.1-21.9 0-43.3 1.4-63.8 4.1-.2-1.3-.2-2.7-.2-4.1zM432 353l0-46.2c15.1-3.9 29.3-8.5 42.2-13.9 13.2-5.5 26.1-12.2 37.8-20.3l0 15.4c0 26.8-31.5 50.5-80 65zm0-96l0-33c0-4.5-.4-8.8-1-13 15.5-3.9 30-8.6 43.2-14.2s26.1-12.2 37.8-20.3l0 15.4c0 26.8-31.5 50.5-80 65zM0 240l0-16c0-44.2 86-80 192-80s192 35.8 192 80l0 16c0 44.2-86 80-192 80S0 284.2 0 240zm384 96c0 44.2-86 80-192 80S0 380.2 0 336l0-15.4c11.6 8.1 24.5 14.7 37.8 20.3 41.9 17.5 96.5 27.1 154.2 27.1s112.3-9.7 154.2-27.1c13.2-5.5 26.1-12.2 37.8-20.3l0 15.4zm0 80.6l0 15.4c0 44.2-86 80-192 80S0 476.2 0 432l0-15.4c11.6 8.1 24.5 14.7 37.8 20.3 41.9 17.5 96.5 27.1 154.2 27.1s112.3-9.7 154.2-27.1c13.2-5.5 26.1-12.2 37.8-20.3z";
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
    exports.faCoins = exports.definition;
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

// src/_entries/faCoins.js
var import_faCoins = __toESM(require_faCoins());
var export_faCoins = import_faCoins.definition;
export {
  export_faCoins as faCoins
};
