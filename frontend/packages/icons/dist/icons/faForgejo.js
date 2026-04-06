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

// src/node_modules/@fortawesome/free-brands-svg-icons/faForgejo.js
var require_faForgejo = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faForgejo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "forgejo";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "e860";
    var svgPathData = "M261.5 1.2c34 0 61.6 27.6 61.6 61.6s-27.6 61.6-61.6 61.6c-23 0-43.1-12.7-53.7-31.4l-28.5 0c-24 0-47 9.5-64 26.5s-26.5 40-26.5 64l0 43.5c26-19.5 57.7-30.2 90.6-30.2l28.5 0c10.6-18.7 30.6-31.4 53.7-31.4 34 0 61.6 27.6 61.6 61.6s-27.6 61.6-61.6 61.6c-23 0-43.1-12.7-53.7-31.4l-28.5 0c-24 0-47 9.5-64 26.5s-26.5 40-26.5 64l0 47.8c18.7 10.6 31.4 30.6 31.4 53.6 0 34-27.6 61.6-61.6 61.6S-3 483.2-3 449.2c0-23 12.7-43.1 31.4-53.6l0-212c0-40 15.9-78.4 44.2-106.7s66.7-44.2 106.7-44.2l28.5 0c10.6-18.7 30.6-31.4 53.7-31.4zM58.6 423.9c-14 0-25.3 11.3-25.3 25.3s11.3 25.4 25.3 25.4 25.4-11.4 25.4-25.4-11.4-25.3-25.4-25.3zM261.5 201.7a25.4 25.4 0 1 0 0 50.7 25.4 25.4 0 1 0 0-50.7zm0-164.2a25.4 25.4 0 1 0 0 50.7 25.4 25.4 0 1 0 0-50.7z";
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
    exports.faForgejo = exports.definition;
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

// src/_entries/faForgejo.js
var import_faForgejo = __toESM(require_faForgejo());
var export_faForgejo = import_faForgejo.definition;
export {
  export_faForgejo as faForgejo
};
