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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTurnLeft.js
var require_faTurnLeft = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTurnLeft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "turn-left";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e636";
    var svgPathData = "M177.5 66c-8.8-3.8-19-2-26 4.6l-144 136C2.7 211.1 0 217.4 0 224s2.7 12.9 7.5 17.4l144 136c7 6.6 17.2 8.4 26 4.6s14.5-12.5 14.5-22l0-88 192 0c17.7 0 32 14.3 32 32l0 144c0 17.7 14.3 32 32 32l32 0c17.7 0 32-14.3 32-32l0-144c0-70.7-57.3-128-128-128l-192 0 0-88c0-9.6-5.7-18.2-14.5-22z";
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
    exports.faTurnLeft = exports.definition;
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

// src/_entries/faTurnLeft.js
var import_faTurnLeft = __toESM(require_faTurnLeft());
var export_faTurnLeft = import_faTurnLeft.definition;
export {
  export_faTurnLeft as faTurnLeft
};
