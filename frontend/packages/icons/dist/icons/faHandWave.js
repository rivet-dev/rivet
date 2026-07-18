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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHandWave.js
var require_faHandWave = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHandWave.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-wave";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e1a7";
    var svgPathData = "M352.2 82.3L351 73.8c-3-21.1-19.6-37.7-40.7-40.7l-8.5-1.2c-8.7-1.2-14.8-9.4-13.6-18.1S297.5-1.1 306.3 .2l8.5 1.2c35.2 5 62.9 32.7 67.9 67.9l1.2 8.5c1.2 8.7-4.8 16.9-13.6 18.1s-16.9-4.8-18.1-13.6zM185.4 70.6c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l192 192c3.3 3.3 9.4 1.1 9.4-3.7l0-77.7c0-22.1 17.9-40 40-40s40 17.9 40 40l0 176.3c0 60.2-27.9 116.9-75.5 153.7C385.7 505.3 321 515.7 263 498.7c-28.4-7.1-55.3-21.8-77.6-44.1l-4.9-4.9L73.4 342.6l-32-32c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l31.9 31.9 .1 .1L165.3 344c5.2 5.2 13.6 5.2 18.7 0s5.2-13.6 0-18.7L57.4 198.6c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L229.3 280c5.2 5.2 13.6 5.2 18.7 0s5.2-13.6 0-18.7L105.4 118.6c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L293.3 216c5.2 5.2 13.6 5.2 18.7 0s5.2-13.6 0-18.7L185.4 70.6zM13.7 352.2c8.7-1.3 16.9 4.8 18.1 13.6l1.2 8.5c3 21.1 19.6 37.7 40.7 40.7l8.5 1.2c8.7 1.2 14.8 9.4 13.6 18.1s-9.4 14.8-18.1 13.6l-8.5-1.2c-35.2-5-62.9-32.7-67.9-67.9L.2 370.3c-1.2-8.7 4.8-16.9 13.6-18.1z";
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
    exports.faHandWave = exports.definition;
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

// src/_entries/faHandWave.js
var import_faHandWave = __toESM(require_faHandWave());
var export_faHandWave = import_faHandWave.definition;
export {
  export_faHandWave as faHandWave
};
