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

// src/node_modules/@fortawesome/free-brands-svg-icons/faMegaport.js
var require_faMegaport = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faMegaport.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "megaport";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5a3";
    var svgPathData = "M222.5 209.6l0 66.2 33.5 33.5 33.3-33.3 0-66.4-33.4-33.4-33.4 33.4zM256 8a248 248 0 1 0 0 496 248 248 0 1 0 0-496zM401.1 422.4l-26.1 19.2-26-19.2 0-65.5-33.4-33.4-33.4 33.4 0 65.5-26.2 19.2-26.1-19.2 0-65.5-33.4-33.4-33.5 33.4 0 65.5-26.1 19.2-26.1-19.2 0-87 59.5-59.5 0-87.9 59.5-59.5 0-75.6 26.1-19.2 26.1 19.2 0 75.6 59.5 59.5 0 87.6 59.7 59.7 0 87.1-.1 0z";
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
    exports.faMegaport = exports.definition;
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

// src/_entries/faMegaport.js
var import_faMegaport = __toESM(require_faMegaport());
var export_faMegaport = import_faMegaport.definition;
export {
  export_faMegaport as faMegaport
};
