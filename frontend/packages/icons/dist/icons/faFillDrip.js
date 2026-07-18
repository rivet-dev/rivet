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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFillDrip.js
var require_faFillDrip = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFillDrip.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fill-drip";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f576";
    var svgPathData = "M309.7 71.6l-64.4 64.4 33.4 33.4c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0l-33.4-33.4-96.4 96.4c-2.9 2.9-5.1 6.5-6.3 10.3l321.5 0 53.7-53.7c4.9-4.9 7.6-11.5 7.6-18.3s-2.7-13.5-7.6-18.3L346.3 71.6C341.5 66.7 334.9 64 328 64s-13.5 2.7-18.3 7.6zM58.3 232.4l96.4-96.4-49.4-49.4c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L200 90.7 264.4 26.3C281.3 9.5 304.1 0 328 0s46.7 9.5 63.6 26.3L517.7 152.4C534.5 169.3 544 192.1 544 216s-9.5 46.7-26.3 63.6L311.6 485.7C294.7 502.5 271.9 512 248 512s-46.7-9.5-63.6-26.3L58.3 359.6C41.5 342.7 32 319.9 32 296s9.5-46.7 26.3-63.6zM512 544c-35.3 0-64-28.7-64-64 0-25.2 32.6-79.6 51.2-108.7 6-9.4 19.5-9.4 25.5 0 18.7 29.1 51.2 83.5 51.2 108.7 0 35.3-28.7 64-64 64z";
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
    exports.faFillDrip = exports.definition;
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

// src/_entries/faFillDrip.js
var import_faFillDrip = __toESM(require_faFillDrip());
var export_faFillDrip = import_faFillDrip.definition;
export {
  export_faFillDrip as faFillDrip
};
