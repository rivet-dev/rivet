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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPresentationScreen.js
var require_faPresentationScreen = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPresentationScreen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "presentation-screen";
    var width = 576;
    var height = 512;
    var aliases = ["presentation"];
    var unicode = "f685";
    var svgPathData = "M32 0C14.3 0 0 14.3 0 32S14.3 64 32 64l0 224c0 35.3 28.7 64 64 64l160 0 0 34.7-70.6 70.6c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L288 445.3l57.4 57.4c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3L320 386.7l0-34.7 160 0c35.3 0 64-28.7 64-64l0-224c17.7 0 32-14.3 32-32s-14.3-32-32-32L512 0 64 0 32 0zM96 64l384 0 0 224-192 0L96 288 96 64z";
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
    exports.faPresentationScreen = exports.definition;
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

// src/_entries/faPresentation.js
var import_faPresentationScreen = __toESM(require_faPresentationScreen());
var export_faPresentation = import_faPresentationScreen.definition;
export {
  export_faPresentation as faPresentation
};
