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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSleeping.js
var require_faFaceSleeping = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSleeping.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-sleeping";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e38d";
    var svgPathData = "M512 256c0-46.6-12.5-90.3-34.2-128L432 128c-19.4 0-36.9-11.7-44.3-29.6c-7.1-17.1-3.7-36.7 8.6-50.4c-2.9-3.2-5.4-6.9-7.3-10.8C350.2 13.6 304.7 0 256 0C114.6 0 0 114.6 0 256S114.6 512 256 512s256-114.6 256-256zM272 96c-8.8 0-16-7.2-16-16s7.2-16 16-16l64 0c6.5 0 12.3 3.9 14.8 9.9s1.1 12.9-3.5 17.4L310.6 128l25.4 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-64 0c-6.5 0-12.3-3.9-14.8-9.9s-1.1-12.9 3.5-17.4L297.4 96 272 96zM216 384a40 40 0 1 1 80 0 40 40 0 1 1 -80 0zM387.2 238.4c5.3-7.1 15.3-8.5 22.4-3.2s8.5 15.3 3.2 22.4c-30.4 40.5-91.2 40.5-121.6 0c-5.3-7.1-3.9-17.1 3.2-22.4s17.1-3.9 22.4 3.2c17.6 23.5 52.8 23.5 70.4 0zm-262.4 0c17.6 23.5 52.8 23.5 70.4 0c5.3-7.1 15.3-8.5 22.4-3.2s8.5 15.3 3.2 22.4c-30.4 40.5-91.2 40.5-121.6 0c-5.3-7.1-3.9-17.1 3.2-22.4s17.1-3.9 22.4 3.2zM416 16c0 8.8 7.2 16 16 16l25.4 0L420.7 68.7c-4.6 4.6-5.9 11.5-3.5 17.4s8.3 9.9 14.8 9.9l64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-25.4 0 36.7-36.7c4.6-4.6 5.9-11.5 3.5-17.4S502.5 0 496 0L432 0c-8.8 0-16 7.2-16 16z";
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
    exports.faFaceSleeping = exports.definition;
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

// src/_entries/faFaceSleeping.js
var import_faFaceSleeping = __toESM(require_faFaceSleeping());
var export_faFaceSleeping = import_faFaceSleeping.definition;
export {
  export_faFaceSleeping as faFaceSleeping
};
