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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRuler.js
var require_faRuler = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRuler.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ruler";
    var width = 576;
    var height = 512;
    var aliases = [128207];
    var unicode = "f545";
    var svgPathData = "M209.1 516.2c-18.7 18.7-49.1 18.7-67.9 0L28.1 403.1c-18.7-18.7-18.7-49.1 0-67.9l17-17 73.5 73.5c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-73.5-73.5 33.9-33.9 50.9 50.9c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-50.9-50.9 33.9-33.9 73.5 73.5c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-73.5-73.5 33.9-33.9 50.9 50.9c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-50.9-50.9 33.9-33.9 73.5 73.5c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-73.5-73.5 17-17c18.7-18.7 49.1-18.7 67.9 0L548.5 108.9c18.7 18.7 18.7 49.1 0 67.9L209.1 516.2z";
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
    exports.faRuler = exports.definition;
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

// src/_entries/faRuler.js
var import_faRuler = __toESM(require_faRuler());
var export_faRuler = import_faRuler.definition;
export {
  export_faRuler as faRuler
};
