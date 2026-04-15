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

// src/node_modules/@fortawesome/free-brands-svg-icons/faEllo.js
var require_faEllo = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faEllo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "ello";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5f1";
    var svgPathData = "M256 8a248 248 0 1 0 0 496 248 248 0 1 0 0-496zM399.8 293.2C383.3 358.5 323.8 404.8 256 404.8S128.7 358.5 112.2 293.2c-1.6-7.4 2.5-15.7 9.9-17.4s15.7 2.5 17.4 9.9c14 52.9 62 90.1 116.6 90.1s102.5-37.2 116.6-90.1c1.7-7.4 9.9-12.4 17.4-9.9 7.4 1.7 12.4 9.9 9.9 17.4z";
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
    exports.faEllo = exports.definition;
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

// src/_entries/faEllo.js
var import_faEllo = __toESM(require_faEllo());
var export_faEllo = import_faEllo.definition;
export {
  export_faEllo as faEllo
};
