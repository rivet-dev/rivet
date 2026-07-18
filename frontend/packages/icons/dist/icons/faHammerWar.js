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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHammerWar.js
var require_faHammerWar = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHammerWar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hammer-war";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f6e4";
    var svgPathData = "M192 0c17.7 0 32 14.3 32 32l0 18.8L347.3 32.4c9.2-1.4 18.6 1.3 25.6 7.4S384 54.7 384 64l0 192c0 9.3-4.1 18.2-11.1 24.2s-16.4 8.8-25.6 7.4L196.7 265.1c-3.1-.5-6.3-.5-9.5 0L36.7 287.6c-9.2 1.4-18.6-1.3-25.6-7.4S0 265.3 0 256L0 64c0-9.3 4.1-18.2 11.1-24.2s16.4-8.8 25.6-7.4L160 50.8 160 32c0-17.7 14.3-32 32-32zm0 296.7l32 4.8L224 480c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-178.5 32-4.8z";
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
    exports.faHammerWar = exports.definition;
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

// src/_entries/faHammerWar.js
var import_faHammerWar = __toESM(require_faHammerWar());
var export_faHammerWar = import_faHammerWar.definition;
export {
  export_faHammerWar as faHammerWar
};
