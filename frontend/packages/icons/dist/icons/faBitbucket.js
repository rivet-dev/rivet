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

// src/node_modules/@fortawesome/free-brands-svg-icons/faBitbucket.js
var require_faBitbucket = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faBitbucket.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "bitbucket";
    var width = 512;
    var height = 512;
    var aliases = [61810];
    var unicode = "f171";
    var svgPathData = "M22.2 32c-2.1 0-4.2 .4-6.1 1.1s-3.7 1.9-5.2 3.4-2.7 3.2-3.5 5.1-1.3 4-1.3 6.1c0 .9 .1 1.9 .2 2.8L74.1 462.7c.8 5.1 3.4 9.7 7.3 13s8.8 5.2 14 5.2l325.7 0c3.8 .1 7.5-1.3 10.5-3.7s4.9-5.9 5.5-9.7L505 50.7c.7-4.2-.3-8.4-2.8-11.9s-6.2-5.7-10.4-6.4c-.9-.1-1.9-.2-2.8-.2L22.2 32zM308.1 329.8l-104 0-28.1-147 157.3 0-25.2 147z";
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
    exports.faBitbucket = exports.definition;
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

// src/_entries/faBitbucket.js
var import_faBitbucket = __toESM(require_faBitbucket());
var export_faBitbucket = import_faBitbucket.definition;
export {
  export_faBitbucket as faBitbucket
};
