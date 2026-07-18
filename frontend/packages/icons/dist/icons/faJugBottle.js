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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faJugBottle.js
var require_faJugBottle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faJugBottle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "jug-bottle";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5fb";
    var svgPathData = "M120 0C106.7 0 96 10.7 96 24l0 24-8 0C74.7 48 64 58.7 64 72s10.7 24 24 24l144 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-8 0 0-24c0-13.3-10.7-24-24-24L120 0zm8 128C57.3 128 0 185.3 0 256L0 448c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-192c0-70.7-57.3-128-128-128l-128 0zm160 96c17.7 0 32 14.3 32 32l0 96c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-96c0-17.7 14.3-32 32-32zM472 64c-13.3 0-24 10.7-24 24l0 40 128 0 0-40c0-13.3-10.7-24-24-24l-80 0zM416.4 392.4l-.4 .2 0 55.4c0 16.9-4.3 32.7-12 46.5c11.5 10.9 27 17.5 44 17.5l128 0c35.3 0 64-28.7 64-64c0-24.1-13.3-45.1-33-56c19.7-10.9 33-31.9 33-56c0-23.5-12.6-44-31.5-55.1c18.7-10.1 31.5-29.9 31.5-52.7c0-23.2-13.4-44.3-34.4-54.2l-13.2-6.2c-11-5.1-23-7.8-35.1-7.8l-90.6 0c-12.1 0-24.1 2.7-35.1 7.8L418.4 174c-6.9 3.2-13 7.7-18 13c10 20.9 15.6 44.3 15.6 69l0 135.4c.3 .2 .7 .4 1 .6c-.2 .1-.4 .2-.6 .4z";
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
    exports.faJugBottle = exports.definition;
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

// src/_entries/faJugBottle.js
var import_faJugBottle = __toESM(require_faJugBottle());
var export_faJugBottle = import_faJugBottle.definition;
export {
  export_faJugBottle as faJugBottle
};
