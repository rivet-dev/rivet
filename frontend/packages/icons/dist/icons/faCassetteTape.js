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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCassetteTape.js
var require_faCassetteTape = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCassetteTape.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cassette-tape";
    var width = 576;
    var height = 512;
    var aliases = [128429];
    var unicode = "f8ab";
    var svgPathData = "M0 96C0 60.7 28.7 32 64 32l448 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64l-28.9 0-36.3-96.9c-7-18.7-24.9-31.1-44.9-31.1l-227.6 0c-20 0-37.9 12.4-44.9 31.1L92.9 480 64 480c-35.3 0-64-28.7-64-64L0 96zM416.8 394.4L448.9 480l-321.8 0 32.1-85.6c2.3-6.2 8.3-10.4 15-10.4l227.6 0c6.7 0 12.6 4.1 15 10.4zM341.5 192l-106.9 0c3.5 10 5.5 20.8 5.5 32s-1.9 22-5.5 32l106.9 0c-3.5-10-5.5-20.8-5.5-32s1.9-22 5.5-32zm90.5 0a32 32 0 1 0 0 64 32 32 0 1 0 0-64zM112 224a32 32 0 1 0 64 0 32 32 0 1 0 -64 0z";
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
    exports.faCassetteTape = exports.definition;
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

// src/_entries/faCassetteTape.js
var import_faCassetteTape = __toESM(require_faCassetteTape());
var export_faCassetteTape = import_faCassetteTape.definition;
export {
  export_faCassetteTape as faCassetteTape
};
