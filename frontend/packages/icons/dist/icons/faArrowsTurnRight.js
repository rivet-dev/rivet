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

// src/node_modules/@fortawesome/free-solid-svg-icons/faArrowsTurnRight.js
var require_faArrowsTurnRight = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faArrowsTurnRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrows-turn-right";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e4c0";
    var svgPathData = "M313.4-6.6c12.5-12.5 32.8-12.5 45.3 0l80 80c12.5 12.5 12.5 32.8 0 45.3l-80 80c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3L338.7 128 128 128c-35.3 0-64 28.7-64 64l0 32c0 17.7-14.3 32-32 32S0 241.7 0 224l0-32C0 121.3 57.3 64 128 64l210.7 0-25.4-25.4c-12.5-12.5-12.5-32.8 0-45.3zm-96 256c12.5-12.5 32.8-12.5 45.3 0l80 80c12.5 12.5 12.5 32.8 0 45.3l-80 80c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3L242.7 384 96 384c-17.7 0-32 14.3-32 32l0 32c0 17.7-14.3 32-32 32S0 465.7 0 448l0-32c0-53 43-96 96-96l146.7 0-25.4-25.4c-12.5-12.5-12.5-32.8 0-45.3z";
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
    exports.faArrowsTurnRight = exports.definition;
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

// src/_entries/faArrowsTurnRight.js
var import_faArrowsTurnRight = __toESM(require_faArrowsTurnRight());
var export_faArrowsTurnRight = import_faArrowsTurnRight.definition;
export {
  export_faArrowsTurnRight as faArrowsTurnRight
};
