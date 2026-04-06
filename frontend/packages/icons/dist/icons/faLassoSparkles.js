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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLassoSparkles.js
var require_faLassoSparkles = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLassoSparkles.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lasso-sparkles";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e1c9";
    var svgPathData = "M290.7 0L288 0C128.9 0 0 78.8 0 176c0 57.2 44.7 108.1 113.9 140.2l-.2 .4L160.5 340c19.3 9.7 31.5 29.4 31.5 51c0 31.5-25.5 57-57 57l-71 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l71 0c66.8 0 121-54.2 121-121c0-14.1-2.4-27.8-7-40.6c12.8 1.1 25.8 1.6 39 1.6c159.1 0 288-78.8 288-176c0-76.4-79.6-141.4-191-165.8l5.6 15.1 26 9.6C435.5 42 448 59.9 448 80c0 5.6-1 11.1-2.8 16.2C486.4 116.5 512 144.8 512 176c0 61.9-100.3 112-224 112S64 237.9 64 176c0-51 68.2-94 161.4-107.6c3.8-15.2 14.8-27.9 29.9-33.4l26-9.6L290.7 0zM351 10.4C348.7 4.2 342.7 0 336 0s-12.7 4.2-15 10.4L306.3 50.3 266.4 65c-6.3 2.3-10.4 8.3-10.4 15s4.2 12.7 10.4 15l39.8 14.7L321 149.6c2.3 6.3 8.3 10.4 15 10.4s12.7-4.2 15-10.4l14.7-39.8L405.6 95c6.3-2.3 10.4-8.3 10.4-15s-4.2-12.7-10.4-15L365.7 50.3 351 10.4zM496 352c-6.7 0-12.7 4.2-15 10.4l-14.7 39.8L426.4 417c-6.3 2.3-10.4 8.3-10.4 15s4.2 12.7 10.4 15l39.8 14.7L481 501.6c2.3 6.3 8.3 10.4 15 10.4s12.7-4.2 15-10.4l14.7-39.8L565.6 447c6.3-2.3 10.4-8.3 10.4-15s-4.2-12.7-10.4-15l-39.8-14.7L511 362.4c-2.3-6.3-8.3-10.4-15-10.4z";
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
    exports.faLassoSparkles = exports.definition;
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

// src/_entries/faLassoSparkles.js
var import_faLassoSparkles = __toESM(require_faLassoSparkles());
var export_faLassoSparkles = import_faLassoSparkles.definition;
export {
  export_faLassoSparkles as faLassoSparkles
};
