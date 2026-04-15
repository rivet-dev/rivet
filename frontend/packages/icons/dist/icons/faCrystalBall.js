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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCrystalBall.js
var require_faCrystalBall = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCrystalBall.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "crystal-ball";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e362";
    var svgPathData = "M448 224c0 62.7-25.7 119.3-67.2 160L67.2 384C25.7 343.3 0 286.7 0 224C0 100.3 100.3 0 224 0S448 100.3 448 224zM152.5 84.3l-9.8 26.4-26.4 9.8c-7 2.6-7 12.4 0 15l26.4 9.8 9.8 26.4c2.6 7 12.4 7 15 0l9.8-26.4 26.4-9.8c7-2.6 7-12.4 0-15l-26.4-9.8-9.8-26.4c-2.6-7-12.4-7-15 0zm159 48c-2.6-7-12.4-7-15 0l-18.4 49.8-49.8 18.4c-7 2.6-7 12.4 0 15l49.8 18.4 18.4 49.8c2.6 7 12.4 7 15 0l18.4-49.8 49.8-18.4c7-2.6 7-12.4 0-15l-49.8-18.4-18.4-49.8zm98.9 331.8c3.7 5.1 5.6 11.3 5.6 17.6c0 16.7-13.6 30.3-30.3 30.3L62.3 512C45.6 512 32 498.4 32 481.7c0-6.3 2-12.5 5.6-17.6L72 416l304 0 34.4 48.1z";
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
    exports.faCrystalBall = exports.definition;
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

// src/_entries/faCrystalBall.js
var import_faCrystalBall = __toESM(require_faCrystalBall());
var export_faCrystalBall = import_faCrystalBall.definition;
export {
  export_faCrystalBall as faCrystalBall
};
