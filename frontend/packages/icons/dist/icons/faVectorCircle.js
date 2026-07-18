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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faVectorCircle.js
var require_faVectorCircle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faVectorCircle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "vector-circle";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2c6";
    var svgPathData = "M192 32c0-17.7 14.3-32 32-32l64 0c17.7 0 32 14.3 32 32l0 9.3C392.2 62.8 449.2 119.8 470.7 192l9.3 0c17.7 0 32 14.3 32 32l0 64c0 17.7-14.3 32-32 32l-9.3 0C449.2 392.2 392.2 449.2 320 470.7l0 9.3c0 17.7-14.3 32-32 32l-64 0c-17.7 0-32-14.3-32-32l0-9.3C119.8 449.2 62.8 392.2 41.3 320L32 320c-17.7 0-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32l9.3 0C62.8 119.8 119.8 62.8 192 41.3l0-9.3zM403.7 194.4c-16.2-38.9-47.3-70-86.2-86.2C312.7 119.9 301.3 128 288 128l-64 0c-13.3 0-24.7-8.1-29.6-19.7c-38.9 16.2-70 47.3-86.2 86.2c11.6 4.8 19.7 16.2 19.7 29.6l0 64c0 13.3-8.1 24.7-19.7 29.6c16.2 38.9 47.3 70 86.2 86.2c4.8-11.6 16.2-19.7 29.6-19.7l64 0c13.3 0 24.7 8.1 29.6 19.7c38.9-16.2 70-47.3 86.2-86.2C392.1 312.7 384 301.3 384 288l0-64c0-13.3 8.1-24.7 19.7-29.6zM240 48l0 32 32 0 0-32-32 0zm32 384l-32 0 0 32 32 0 0-32zM432 240l0 32 32 0 0-32-32 0zM80 240l-32 0 0 32 32 0 0-32z";
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
    exports.faVectorCircle = exports.definition;
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

// src/_entries/faVectorCircle.js
var import_faVectorCircle = __toESM(require_faVectorCircle());
var export_faVectorCircle = import_faVectorCircle.definition;
export {
  export_faVectorCircle as faVectorCircle
};
