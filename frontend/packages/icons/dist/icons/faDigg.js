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

// src/node_modules/@fortawesome/free-brands-svg-icons/faDigg.js
var require_faDigg = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faDigg.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "digg";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f1a6";
    var svgPathData = "M81.7 172.3l-81.7 0 0 174.4 132.7 0 0-250.7-51 0 0 76.3zm0 133.4l-30.8 0 0-92.3 30.8 0 0 92.3zM378.9 172.3l0 174.4 81.8 0 0 28.5-81.8 0 0 40.8 133.1 0 0-243.7-133.1 0zm81.8 133.4l-30.8 0 0-92.3 30.8 0 0 92.3zm-235.6 41l82.1 0 0 28.5-82.1 0 0 40.8 133.3 0 0-243.7-133.3 0 0 174.4zm51.2-133.3l30.8 0 0 92.3-30.8 0 0-92.3zM153.3 96l51.3 0 0 51-51.3 0 0-51zm0 76.3l51.3 0 0 174.4-51.3 0 0-174.4z";
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
    exports.faDigg = exports.definition;
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

// src/_entries/faDigg.js
var import_faDigg = __toESM(require_faDigg());
var export_faDigg = import_faDigg.definition;
export {
  export_faDigg as faDigg
};
