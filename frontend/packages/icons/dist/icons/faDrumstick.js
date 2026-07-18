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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDrumstick.js
var require_faDrumstick = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDrumstick.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "drumstick";
    var width = 512;
    var height = 512;
    var aliases = [127831];
    var unicode = "f6d6";
    var svgPathData = "M150.6 287.8c6-6 9.4-14.1 9.4-22.6l0-89.2C160 78.8 238.8 0 336 0s176 78.8 176 176s-78.8 176-176 176l-89.2 0c-8.5 0-16.6 3.4-22.6 9.4l-26.8 26.8c-12.3 12.3-11.4 32.5-7.2 49.4c1.1 4.6 1.8 9.5 1.8 14.5c0 33.1-26.9 60-60 60s-60-26.9-60-60c0-6.3-5.7-12-12-12c-33.1 0-60-26.9-60-60s26.9-60 60-60c5 0 9.8 .6 14.5 1.8c16.9 4.2 37.1 5.2 49.4-7.2l26.8-26.8z";
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
    exports.faDrumstick = exports.definition;
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

// src/_entries/faDrumstick.js
var import_faDrumstick = __toESM(require_faDrumstick());
var export_faDrumstick = import_faDrumstick.definition;
export {
  export_faDrumstick as faDrumstick
};
