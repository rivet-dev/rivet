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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDrawCircle.js
var require_faDrawCircle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDrawCircle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "draw-circle";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5ed";
    var svgPathData = "M472.1 196.7c-20.8-76-80.7-135.9-156.7-156.7C305.8 16.5 282.8 0 256 0s-49.8 16.5-59.3 39.9c-76 20.8-135.9 80.7-156.7 156.7C16.5 206.2 0 229.2 0 256s16.5 49.8 39.9 59.3c20.8 76 80.7 135.9 156.7 156.7c9.5 23.4 32.5 39.9 59.3 39.9s49.8-16.5 59.3-39.9c76-20.8 135.9-80.7 156.7-156.7c23.4-9.5 39.9-32.5 39.9-59.3s-16.5-49.8-39.9-59.3zM408 206c-14.7 11.7-24 29.8-24 50s9.4 38.3 24 50c-15.8 48.2-53.9 86.2-102 102c-11.7-14.7-29.8-24-50-24s-38.3 9.4-50 24c-48.2-15.8-86.2-53.9-102-102c14.7-11.7 24-29.8 24-50s-9.4-38.3-24-50c15.8-48.2 53.9-86.2 102-102c11.7 14.7 29.8 24 50 24s38.3-9.4 50-24c48.2 15.8 86.2 53.9 102 102z";
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
    exports.faDrawCircle = exports.definition;
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

// src/_entries/faDrawCircle.js
var import_faDrawCircle = __toESM(require_faDrawCircle());
var export_faDrawCircle = import_faDrawCircle.definition;
export {
  export_faDrawCircle as faDrawCircle
};
