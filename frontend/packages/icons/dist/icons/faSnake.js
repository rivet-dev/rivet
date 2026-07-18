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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSnake.js
var require_faSnake = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSnake.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "snake";
    var width = 512;
    var height = 512;
    var aliases = [128013];
    var unicode = "f716";
    var svgPathData = "M483.4 65.7L403.6 25.8c-30.8-15.4-68-9.4-92.3 15L288 64 176 64C96.5 64 32 128.5 32 208s64.5 144 144 144l144 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-63.9 0-.1 0-144.4 0c-10.3 0-20.6 1.7-30.4 4.9l-66 22C6.1 446 0 454.4 0 464s6.1 18 15.2 21.1l66 22c9.8 3.3 20 4.9 30.4 4.9l64.3 0 .1 0 144 0c70.7 0 128-57.3 128-128s-57.3-128-128-128l-144 0c-26.5 0-48-21.5-48-48s21.5-48 48-48l112 0 23.2 23.2c24.3 24.3 61.5 30.4 92.3 15l79.8-39.9c17.5-8.8 28.6-26.7 28.6-46.3s-11.1-37.5-28.6-46.3zM352 80a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm16 48a16 16 0 1 1 0 32 16 16 0 1 1 0-32z";
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
    exports.faSnake = exports.definition;
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

// src/_entries/faSnake.js
var import_faSnake = __toESM(require_faSnake());
var export_faSnake = import_faSnake.definition;
export {
  export_faSnake as faSnake
};
