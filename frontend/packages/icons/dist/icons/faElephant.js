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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faElephant.js
var require_faElephant = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faElephant.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "elephant";
    var width = 640;
    var height = 512;
    var aliases = [128024];
    var unicode = "f6da";
    var svgPathData = "M304 0C242.1 0 192 50.1 192 112s50.1 112 112 112c8.8 0 16 7.2 16 16s-7.2 16-16 16c-79.5 0-144-64.5-144-144c0-25.3 6.5-49.1 18-69.8c-20.5 7-39.4 17.3-56.1 30.4C67.4 111.9 32 175.8 32 248l0 19c0 12.4-1.2 24.7-3.7 36.9c-1.9 9.3-5.8 18-11.4 25.6L4.8 345.6c-8 10.6-5.8 25.6 4.8 33.6s25.6 5.8 33.6-4.8l12.1-16.2c3.2-4.3 6.1-8.8 8.7-13.5L64 480c0 17.7 14.3 32 32 32l64 0c17.7 0 32-14.3 32-32l0-70c15.3 3.9 31.4 6 48 6s32.7-2.1 48-6l0 70c0 17.7 14.3 32 32 32l64 0c17.7 0 32-14.3 32-32l0-139.3c16.7 18.9 38.8 32.8 64 39.3l0 18.6c0 45 36.5 81.4 81.4 81.4c30.8 0 59-17.4 72.8-45l2.3-4.7c7.9-15.8 1.5-35-14.3-42.9s-35-1.5-42.9 14.3l-2.3 4.7c-3 5.9-9 9.6-15.6 9.6c-9.6 0-17.4-7.8-17.4-17.4l0-18.6c22.9-5.9 43.4-18 59.4-34.4c6.2-6.3 6.1-16.4-.2-22.6c-6.3-6.2-16.3-6.1-22.5 .1c-1.3 1.3-2.6 2.6-4 3.9c-1.5 1.4-3.1 2.7-4.6 4c-8.3 6.7-17.8 12-28 15.6c0 0 0 0 0 0c-10 3.5-20.8 5.5-32 5.5s-22-1.9-32-5.5c0 0 0 0 0 0c-7.5-2.6-14.6-6.2-21.1-10.5c0 0 0 0 0 0c-9.1-6.1-17.2-13.7-23.7-22.4c-5.3-7.1-3.9-17.1 3.2-22.4s17.1-3.9 22.4 3.2C472.5 310 491.1 320 512 320c11.7 0 22.6-3.1 32-8.6L544 192c0-88.4-71.6-160-160-160l-1.6 0C362.2 12.2 334.5 0 304 0zM464 192a16 16 0 1 1 0 32 16 16 0 1 1 0-32z";
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
    exports.faElephant = exports.definition;
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

// src/_entries/faElephant.js
var import_faElephant = __toESM(require_faElephant());
var export_faElephant = import_faElephant.definition;
export {
  export_faElephant as faElephant
};
