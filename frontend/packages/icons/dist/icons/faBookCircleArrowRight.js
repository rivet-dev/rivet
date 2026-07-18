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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBookCircleArrowRight.js
var require_faBookCircleArrowRight = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBookCircleArrowRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "book-circle-arrow-right";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e0bc";
    var svgPathData = "M96 0C43 0 0 43 0 96L0 416c0 53 43 96 96 96l288 0 10.8 0c-23.3-16.4-42.5-38.4-55.6-64L96 448c-17.7 0-32-14.3-32-32s14.3-32 32-32l224.7 0c-.5-5.3-.7-10.6-.7-16c0-80.6 54.1-148.5 128-169.4L448 32c0-17.7-14.3-32-32-32L384 0 96 0zM640 368a144 144 0 1 0 -288 0 144 144 0 1 0 288 0zM492.7 300.7c6.2-6.2 16.4-6.2 22.6 0l56 56c6.2 6.2 6.2 16.4 0 22.6l-56 56c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6L521.4 384 432 384c-8.8 0-16-7.2-16-16s7.2-16 16-16l89.4 0-28.7-28.7c-6.2-6.2-6.2-16.4 0-22.6z";
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
    exports.faBookCircleArrowRight = exports.definition;
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

// src/_entries/faBookCircleArrowRight.js
var import_faBookCircleArrowRight = __toESM(require_faBookCircleArrowRight());
var export_faBookCircleArrowRight = import_faBookCircleArrowRight.definition;
export {
  export_faBookCircleArrowRight as faBookCircleArrowRight
};
