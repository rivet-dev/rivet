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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPepperHot.js
var require_faPepperHot = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPepperHot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pepper-hot";
    var width = 576;
    var height = 512;
    var aliases = [127798];
    var unicode = "f816";
    var svgPathData = "M545 65c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0L478.9 63.2C452.3 43.6 419.5 32 384 32 348.9 32 316.5 43.3 290.1 62.4 274.1 74.1 284.2 96 304 96l24 0c13.3 0 24 10.7 24 24l0 80c0 13.3 10.7 24 24 24l80 0c13.3 0 24 10.7 24 24l0 24c0 19.8 21.9 29.9 33.6 13.9 19.1-26.4 30.4-58.8 30.4-93.9 0-35.5-11.6-68.3-31.2-94.9L545 65zM270 135.5l-159.6 228C101.3 376.4 86.7 384 71 384l-7 0c-26.5 0-48 21.5-48 48s21.5 48 48 48l27.1 0c65.7 0 129.7-20.2 183.5-57.8L440.5 306.1C435 295.9 432 284.2 432 272l-56 0c-39.8 0-72-32.2-72-72l0-56c-12.2 0-23.9-3-34-8.5z";
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
    exports.faPepperHot = exports.definition;
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

// src/_entries/faPepperHot.js
var import_faPepperHot = __toESM(require_faPepperHot());
var export_faPepperHot = import_faPepperHot.definition;
export {
  export_faPepperHot as faPepperHot
};
