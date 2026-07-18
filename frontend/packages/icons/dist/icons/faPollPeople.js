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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPollPeople.js
var require_faPollPeople = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPollPeople.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "poll-people";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f759";
    var svgPathData = "M128 48A48 48 0 1 0 32 48a48 48 0 1 0 96 0zm448 80l-96 0 0-32 96 0 0 32zM272 32c-26.5 0-48 21.5-48 48l0 64c0 26.5 21.5 48 48 48l320 0c26.5 0 48-21.5 48-48l0-64c0-26.5-21.5-48-48-48L272 32zM576 416l-224 0 0-32 224 0 0 32zM272 320c-26.5 0-48 21.5-48 48l0 64c0 26.5 21.5 48 48 48l320 0c26.5 0 48-21.5 48-48l0-64c0-26.5-21.5-48-48-48l-320 0zM64 128c-35.3 0-64 28.7-64 64c0 17.7 14.3 32 32 32l96 0c17.7 0 32-14.3 32-32c0-35.3-28.7-64-64-64l-32 0zm64 208a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM64 416c-35.3 0-64 28.7-64 64c0 17.7 14.3 32 32 32l96 0c17.7 0 32-14.3 32-32c0-35.3-28.7-64-64-64l-32 0z";
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
    exports.faPollPeople = exports.definition;
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

// src/_entries/faPollPeople.js
var import_faPollPeople = __toESM(require_faPollPeople());
var export_faPollPeople = import_faPollPeople.definition;
export {
  export_faPollPeople as faPollPeople
};
