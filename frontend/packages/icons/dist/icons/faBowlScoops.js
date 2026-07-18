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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlScoops.js
var require_faBowlScoops = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlScoops.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bowl-scoops";
    var width = 448;
    var height = 512;
    var aliases = [127813];
    var unicode = "e3df";
    var svgPathData = "M224 208c0 5.4 .4 10.8 1.1 16l221.7 0c.7-5.2 1.1-10.6 1.1-16c0-61.9-50.1-112-112-112l-1.1 0c-38.7 .4-72.7 20.4-92.6 50.6C230.7 164.2 224 185.3 224 208zM0 208c0 5.4 .4 10.8 1.1 16l191.7 0c-.6-5.3-.9-10.6-.9-16c0-22 4.9-42.8 13.7-61.4c-13.9-21.2-34.9-37.4-59.6-45.3C135.4 97.9 123.9 96 112 96C50.1 96 0 146.1 0 208zM325.4 64.4C307.5 26.3 268.8 0 224 0s-83.5 26.3-101.4 64.4c40.9 3 77.1 23 101.4 53.1c24.3-30.1 60.5-50.1 101.4-53.1zM16 256c-4.6 0-9 2-12 5.4s-4.5 8-3.9 12.6l2 15.9C11.1 361.9 72.4 416 145 416l40.4 0-68.7 68.7c-4.6 4.6-5.9 11.5-3.5 17.4s8.3 9.9 14.8 9.9l192 0c6.5 0 12.3-3.9 14.8-9.9s1.1-12.9-3.5-17.4L262.6 416l40.4 0c72.6 0 133.9-54.1 142.9-126.1l2-15.9c.6-4.6-.8-9.1-3.9-12.6s-7.4-5.4-12-5.4L16 256z";
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
    exports.faBowlScoops = exports.definition;
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

// src/_entries/faBowlScoops.js
var import_faBowlScoops = __toESM(require_faBowlScoops());
var export_faBowlScoops = import_faBowlScoops.definition;
export {
  export_faBowlScoops as faBowlScoops
};
