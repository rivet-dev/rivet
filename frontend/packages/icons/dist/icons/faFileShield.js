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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFileShield.js
var require_faFileShield = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFileShield.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-shield";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e4f0";
    var svgPathData = "M96 0C60.7 0 32 28.7 32 64l0 384c0 35.3 28.7 64 64 64l201.2 0C261 469.4 240 414.5 240 356.4l0-31.1c0-34.4 22-65 54.7-75.9l112-37.3c3.1-1 6.2-1.8 9.3-2.5l0-39.1c0-17-6.7-33.3-18.7-45.3L290.7 18.7C278.7 6.7 262.5 0 245.5 0L96 0zM357.5 176L264 176c-13.3 0-24-10.7-24-24L240 58.5 357.5 176zm87.8 312.5l-13.3 6.3 0-188.1 96 32 0 19.6c0 55.8-32.2 106.5-82.7 130.3zM421.9 259.5l-112 37.3c-13.1 4.4-21.9 16.6-21.9 30.4l0 31.1c0 74.4 43 142.1 110.2 173.7l18.5 8.7c4.8 2.2 10 3.4 15.2 3.4s10.5-1.2 15.2-3.4l18.5-8.7C533 500.3 576 432.6 576 358.2l0-31.1c0-13.8-8.8-26-21.9-30.4l-112-37.3c-6.6-2.2-13.7-2.2-20.2 0z";
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
    exports.faFileShield = exports.definition;
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

// src/_entries/faFileShield.js
var import_faFileShield = __toESM(require_faFileShield());
var export_faFileShield = import_faFileShield.definition;
export {
  export_faFileShield as faFileShield
};
