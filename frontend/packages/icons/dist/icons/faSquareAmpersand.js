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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareAmpersand.js
var require_faSquareAmpersand = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareAmpersand.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-ampersand";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e260";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM184 191.6c0 4.1 1.6 8.1 4.6 11L203 217.1l23.6-17.3c3.4-2.5 5.4-6.4 5.4-10.6c0-7.3-5.9-13.2-13.2-13.2l-19.2 0c-8.6 0-15.6 7-15.6 15.6zm71 46.9l-17.6 12.9 28.7 28.7 26.2-37.8c7.5-10.9 22.5-13.6 33.4-6.1s13.6 22.5 6.1 33.4l-31.1 45L329 343c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0l-22.4-22.4c-15.1 18.5-37.8 29.4-62 29.4l-32.2 0c-36.7 0-66.5-29.8-66.5-66.5c0-21.2 10.1-41.1 27.2-53.6l24.7-18.1-9.2-9.2c-11.9-11.9-18.6-28.1-18.6-45c0-35.1 28.5-63.6 63.6-63.6l19.2 0c33.8 0 61.2 27.4 61.2 61.2c0 19.5-9.3 37.8-25 49.3zm-87.5 64.1c-4.7 3.5-7.5 9-7.5 14.9c0 10.2 8.3 18.5 18.5 18.5l32.2 0c10.5 0 20.3-5.2 26.3-13.8l1.4-2-40.1-40.1-30.7 22.5z";
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
    exports.faSquareAmpersand = exports.definition;
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

// src/_entries/faSquareAmpersand.js
var import_faSquareAmpersand = __toESM(require_faSquareAmpersand());
var export_faSquareAmpersand = import_faSquareAmpersand.definition;
export {
  export_faSquareAmpersand as faSquareAmpersand
};
