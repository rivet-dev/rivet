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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHatSanta.js
var require_faHatSanta = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHatSanta.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hat-santa";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f7a7";
    var svgPathData = "M149.7 137.8L64 352l384 0L385.1 210.6c-.8-1.7-1.1-3.5-1.1-5.4c0-7.3 5.9-13.2 13.2-13.2l.8 0c1.4 0 2.7 .2 4 .6l55.3 16.6c2.1-3.2 4.5-6.1 7.2-8.8c0-14.3 5.5-28.7 16.4-39.6c2.2-2.2 4.6-4.2 7.1-6L434.4 91.6c-32-37.8-79-59.6-128.5-59.6c-68.8 0-130.7 41.9-156.3 105.8zm434.3 44.4C583 169.8 572.7 160 560 160s-23 9.8-23.9 22.2c-9.4-8.1-23.7-7.7-32.6 1.2s-9.4 23.2-1.2 32.6C489.8 217 480 227.3 480 240s9.8 23 22.2 23.9c-8.1 9.4-7.7 23.7 1.2 32.6s23.2 9.4 32.6 1.2C537 310.2 547.3 320 560 320s23-9.8 23.9-22.2c9.4 8.1 23.7 7.7 32.6-1.2s9.4-23.2 1.2-32.6C630.2 263 640 252.7 640 240s-9.8-23-22.2-23.9c8.1-9.4 7.7-23.7-1.2-32.6s-23.2-9.4-32.6-1.2zM0 432c0 26.5 21.5 48 48 48l416 0c26.5 0 48-21.5 48-48s-21.5-48-48-48L48 384c-26.5 0-48 21.5-48 48z";
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
    exports.faHatSanta = exports.definition;
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

// src/_entries/faHatSanta.js
var import_faHatSanta = __toESM(require_faHatSanta());
var export_faHatSanta = import_faHatSanta.definition;
export {
  export_faHatSanta as faHatSanta
};
