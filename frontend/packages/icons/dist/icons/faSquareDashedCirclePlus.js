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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareDashedCirclePlus.js
var require_faSquareDashedCirclePlus = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareDashedCirclePlus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-dashed-circle-plus";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e5c2";
    var svgPathData = "M96 32C43 32 0 75 0 128l0 32 64 0 0-32c0-17.7 14.3-32 32-32l32 0 0-64L96 32zM0 192L0 320l64 0 0-128L0 192zM64 352L0 352l0 32c0 53 43 96 96 96l32 0 0-64-32 0c-17.7 0-32-14.3-32-32l0-32zM384 128l0 70.6c15.3-4.3 31.4-6.6 48-6.6c5.4 0 10.7 .2 16 .7l0-64.7c0-53-43-96-96-96l-32 0 0 64 32 0c17.7 0 32 14.3 32 32zM160 480l136.2 0c-15.3-18.5-26.9-40.2-33.6-64L160 416l0 64zm0-384l128 0 0-64L160 32l0 64zM432 512a144 144 0 1 0 0-288 144 144 0 1 0 0 288zm16-208l0 48 48 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-48 0 0 48c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-48-48 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l48 0 0-48c0-8.8 7.2-16 16-16s16 7.2 16 16z";
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
    exports.faSquareDashedCirclePlus = exports.definition;
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

// src/_entries/faSquareDashedCirclePlus.js
var import_faSquareDashedCirclePlus = __toESM(require_faSquareDashedCirclePlus());
var export_faSquareDashedCirclePlus = import_faSquareDashedCirclePlus.definition;
export {
  export_faSquareDashedCirclePlus as faSquareDashedCirclePlus
};
