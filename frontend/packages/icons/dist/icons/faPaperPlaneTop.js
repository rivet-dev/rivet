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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPaperPlaneTop.js
var require_faPaperPlaneTop = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPaperPlaneTop.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "paper-plane-top";
    var width = 512;
    var height = 512;
    var aliases = ["paper-plane-alt", "send"];
    var unicode = "e20a";
    var svgPathData = "M57.6 35.8C23.1 20.6-11.3 57.4 6.1 90.9l63 121.2c4.4 8.4 12.6 14.1 22 15.3c0 0 0 0 0 0L266 249.3c3.4 .4 6 3.3 6 6.7s-2.6 6.3-6 6.7L91.1 284.6s0 0 0 0c-9.4 1.2-17.6 6.9-22 15.3L6.1 421.1c-17.4 33.5 17 70.2 51.6 55.1L492.9 285.3c25.5-11.2 25.5-47.4 0-58.6L57.6 35.8z";
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
    exports.faPaperPlaneTop = exports.definition;
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

// src/_entries/faPaperPlaneTop.js
var import_faPaperPlaneTop = __toESM(require_faPaperPlaneTop());
var export_faPaperPlaneTop = import_faPaperPlaneTop.definition;
export {
  export_faPaperPlaneTop as faPaperPlaneTop
};
