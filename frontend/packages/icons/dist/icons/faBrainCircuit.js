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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBrainCircuit.js
var require_faBrainCircuit = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBrainCircuit.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "brain-circuit";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0c6";
    var svgPathData = "M240 56c0-30.9-25.1-56-56-56c-28.9 0-52.7 21.9-55.7 49.9C100.6 56.9 80 82.1 80 112c0 6 .8 11.9 2.4 17.4C53.6 135.7 32 161.3 32 192c0 15 5.1 28.8 13.8 39.7C18.7 244.5 0 272.1 0 304c0 34.2 21.4 63.4 51.6 74.8C49.3 385.4 48 392.6 48 400c0 35.3 28.7 64 64 64c5.6 0 11.1-.7 16.3-2.1c2.9 28.2 26.8 50.1 55.7 50.1c30.9 0 56-25.1 56-56l0-184-48 0c-8.8 0-16 7.2-16 16l0 34.7c18.6 6.6 32 24.4 32 45.3c0 26.5-21.5 48-48 48s-48-21.5-48-48c0-20.9 13.4-38.7 32-45.3l0-34.7c0-26.5 21.5-48 48-48l48 0 0-80-34.7 0c-6.6 18.6-24.4 32-45.3 32c-26.5 0-48-21.5-48-48s21.5-48 48-48c20.9 0 38.7 13.4 45.3 32l34.7 0 0-72zm32 104l0 192 34.7 0c6.6-18.6 24.4-32 45.3-32c26.5 0 48 21.5 48 48s-21.5 48-48 48c-20.9 0-38.7-13.4-45.3-32L272 384l0 72c0 30.9 25.1 56 56 56c28.9 0 52.7-21.9 55.7-50.1c5.2 1.4 10.7 2.1 16.3 2.1c35.3 0 64-28.7 64-64c0-7.4-1.3-14.6-3.6-21.2C490.6 367.4 512 338.2 512 304c0-31.9-18.7-59.5-45.8-72.3C474.9 220.8 480 207 480 192c0-30.7-21.6-56.3-50.4-62.6c1.6-5.5 2.4-11.4 2.4-17.4c0-29.9-20.6-55.1-48.3-62.1C380.6 21.9 356.9 0 328 0c-30.9 0-56 25.1-56 56l0 72 48 0c26.5 0 48 21.5 48 48l0 2.7c18.6 6.6 32 24.4 32 45.3c0 26.5-21.5 48-48 48s-48-21.5-48-48c0-20.9 13.4-38.7 32-45.3l0-2.7c0-8.8-7.2-16-16-16l-48 0zM144 144a16 16 0 1 0 32 0 16 16 0 1 0 -32 0zm208 64a16 16 0 1 0 0 32 16 16 0 1 0 0-32zM144 368a16 16 0 1 0 32 0 16 16 0 1 0 -32 0zm192 0a16 16 0 1 0 32 0 16 16 0 1 0 -32 0z";
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
    exports.faBrainCircuit = exports.definition;
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

// src/_entries/faBrainCircuit.js
var import_faBrainCircuit = __toESM(require_faBrainCircuit());
var export_faBrainCircuit = import_faBrainCircuit.definition;
export {
  export_faBrainCircuit as faBrainCircuit
};
