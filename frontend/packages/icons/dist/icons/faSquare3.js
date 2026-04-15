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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquare3.js
var require_faSquare3 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquare3.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-3";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e258";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zm88 96l128 0c9.9 0 18.8 6.1 22.4 15.3s1.1 19.7-6.2 26.4l-50.8 46.5c41.9 4.8 74.6 40.4 74.6 83.6c0 46.5-37.7 84.2-84.2 84.2L208 384c-30.1 0-58.1-15.6-73.9-41.2l-2.6-4.2c-7-11.3-3.5-26.1 7.8-33s26.1-3.5 33 7.8l2.6 4.2c7.1 11.5 19.6 18.4 33 18.4l27.8 0c20 0 36.2-16.2 36.2-36.2c0-20.1-16.3-36.3-36.4-36.2l-51.5 .3c-9.9 .1-18.9-6-22.5-15.2s-1.2-19.8 6.1-26.5L218.3 176 152 176c-13.3 0-24-10.7-24-24s10.7-24 24-24z";
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
    exports.faSquare3 = exports.definition;
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

// src/_entries/faSquare3.js
var import_faSquare3 = __toESM(require_faSquare3());
var export_faSquare3 = import_faSquare3.definition;
export {
  export_faSquare3 as faSquare3
};
